//! Non-configurable network boundary for this local-only fork.
//!
//! Validate destinations before reading credentials or making requests. Never
//! resolve arbitrary names, follow redirects, or use environment/system proxies.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub const IMPORT_DISABLED: &str =
    "External imports are disabled in this local-only fork. Use a local backup or archive.";

pub fn loopback_ip(host: &str) -> Result<IpAddr, String> {
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    match host.parse::<IpAddr>() {
        Ok(ip) if ip.is_loopback() => Ok(ip),
        _ => Err("Local-only policy: only localhost or a loopback IP address is allowed".into()),
    }
}

pub fn local_url(value: &str) -> Result<reqwest::Url, String> {
    let url =
        reqwest::Url::parse(value).map_err(|_| "Local-only policy: invalid URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("Local-only policy: use an HTTP URL without credentials or a fragment".into());
    }
    loopback_ip(url.host_str().unwrap_or_default())?;
    Ok(url)
}

pub fn validate_server(cfg: &crate::config::Config) -> Result<(), String> {
    loopback_ip(&cfg.server.host)?;
    if let Some(url) = &cfg.server.public_url {
        local_url(url)?;
    }
    for origin in &cfg.server.cors_origins {
        local_url(origin)?;
    }
    if !cfg.server.trusted_proxies.is_empty() || cfg.server.mcp_path_token.is_some() {
        return Err(
            "Local-only policy: trusted proxies and authless MCP path tokens are disabled".into(),
        );
    }
    Ok(())
}

pub fn client() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve("localhost", SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
}

pub fn blocking_client() -> reqwest::blocking::ClientBuilder {
    reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .resolve("localhost", SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
}

/// Reject browser requests from remote sites and DNS-rebinding hostnames before
/// they can reach even login-free APIs. A local reverse proxy is not supported.
pub async fn local_requests_only(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::{
        http::{StatusCode, header},
        response::IntoResponse,
    };
    let headers = request.headers();
    let invalid_host = headers.get(header::HOST).is_some_and(|host| {
        host.to_str()
            .ok()
            .and_then(|h| local_url(&format!("http://{h}")).ok())
            .is_none_or(|url| url.path() != "/" || url.query().is_some())
    });
    let invalid_origin = headers.get(header::ORIGIN).is_some_and(|origin| {
        origin
            .to_str()
            .ok()
            .and_then(|o| local_url(o).ok())
            .is_none_or(|url| url.path() != "/" || url.query().is_some())
    });
    let forwarded = headers
        .keys()
        .any(|name| name.as_str() == "forwarded" || name.as_str().starts_with("x-forwarded-"));
    let cross_site = headers
        .get("sec-fetch-site")
        .is_some_and(|v| v == "cross-site");
    let remote_peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .is_some_and(|peer| !peer.0.ip().is_loopback());
    if invalid_host || invalid_origin || forwarded || cross_site || remote_peer {
        return (
            StatusCode::FORBIDDEN,
            "Local-only policy: remote or proxied requests are disabled",
        )
            .into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_destinations_are_accepted_without_dns() {
        for url in [
            "http://127.0.0.1:3456",
            "http://localhost:3456",
            "http://[::1]:3456",
            "http://127.2.3.4",
        ] {
            assert!(local_url(url).is_ok(), "{url}");
        }
        for url in [
            "https://example.com",
            "https://localhost.example.com",
            "http://localhost.",
            "http://192.168.1.2",
            "http://10.0.0.1",
            "http://0.0.0.0",
            "http://[::]",
            "http://[::ffff:192.168.1.2]",
            "http://169.254.169.254",
            "file:///etc/passwd",
            "http://localhost@evil.example",
            "http://user:secret@localhost",
            "http://localhost/#fragment",
        ] {
            assert!(local_url(url).is_err(), "{url}");
        }
    }

    #[test]
    fn server_cannot_enable_remote_access() {
        let mut cfg = crate::config::Config::default();
        assert!(validate_server(&cfg).is_ok());
        cfg.server.host = "0.0.0.0".into();
        assert!(validate_server(&cfg).is_err());
        cfg.server.host = "127.0.0.1".into();
        cfg.server.public_url = Some("https://example.com".into());
        assert!(validate_server(&cfg).is_err());
        cfg.server.public_url = None;
        cfg.server.cors_origins = vec!["https://example.com".into()];
        assert!(validate_server(&cfg).is_err());
        cfg.server.cors_origins.clear();
        cfg.server.trusted_proxies = vec!["127.0.0.1".into()];
        assert!(validate_server(&cfg).is_err());
        cfg.server.trusted_proxies.clear();
        cfg.server.mcp_path_token = Some("secret".into());
        assert!(validate_server(&cfg).is_err());
    }

    #[tokio::test]
    async fn local_clients_do_not_follow_redirects() {
        use axum::{Router, response::Redirect, routing::get};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/",
            get(|| async { Redirect::temporary("http://192.0.2.1/secret") }),
        );
        let task = tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
        let response = client()
            .build()
            .unwrap()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::TEMPORARY_REDIRECT);
        task.abort();
    }
}
