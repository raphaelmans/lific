//! Non-optional runtime policy for the personal fork.
use crate::{
    config::Config,
    db::{self, DbPool},
    error::LificError,
};
use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

pub fn configure(cfg: &mut Config) {
    cfg.auth.required = false;
    cfg.auth.allow_signup = false;
}

/// Keep historical authors and content intact, but use one local operator.
/// Existing multi-user sessions and publication flags cannot survive a restart.
pub fn prepare(pool: &DbPool) -> Result<String, LificError> {
    pool.transaction(|conn| {
        db::queries::settings::ensure(conn, false)?;
        let operator = match db::queries::users::first_admin(conn)? {
            Some(operator) => operator,
            None => {
                db::queries::users::create_passwordless_admin(conn, "local-operator")?;
                db::queries::users::first_admin(conn)?.ok_or_else(|| {
                    LificError::Internal("Could not initialize the local operator".into())
                })?
            }
        };
        db::queries::settings::update(
            conn,
            db::queries::settings::InstanceSettingsPatch {
                allow_signup: Some(false),
                web_auto_login: Some(true),
                authz_enforced: Some(false),
                ..Default::default()
            },
        )?;
        conn.execute("UPDATE projects SET is_public = 0 WHERE is_public != 0", [])?;
        conn.execute("DELETE FROM sessions WHERE user_id != ?1", [operator.id])?;
        conn.execute(
            "UPDATE api_keys SET revoked = 1 WHERE user_id IS NOT NULL AND user_id != ?1
             AND user_id NOT IN (SELECT id FROM users WHERE is_bot = 1 AND owner_id = ?1)
             AND revoked = 0",
            [operator.id],
        )?;
        Ok(operator.username)
    })
}

fn unavailable(path: &str, method: &Method) -> bool {
    let path = path.trim_end_matches('/');
    if path.starts_with("/api/auth/") {
        return match path {
            "/api/auth/bots" => !matches!(*method, Method::GET | Method::POST),
            "/api/auth/auto-login" | "/api/auth/me" | "/api/auth/me/refresh" => false,
            _ => !path.starts_with("/api/auth/bots/"),
        };
    }
    path.starts_with("/oauth")
        || path.starts_with("/.well-known")
        || path.starts_with("/mcp/")
        || matches!(
            path,
            "/register" | "/authorize" | "/token" | "/revoke" | "/device" | "/login" | "/signup"
        )
        || path == "/public"
        || path.starts_with("/public/")
        || path.starts_with("/api/public/")
        || (path == "/api/users" && method != Method::GET)
        || path.starts_with("/api/users/")
        || (path.starts_with("/api/projects/")
            && path
                .split('/')
                .any(|part| matches!(part, "members" | "import")))
        || (path == "/api/instance/settings" && method != Method::GET)
}

pub async fn requests(
    State(pool): State<DbPool>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    if unavailable(&path, request.method()) {
        return (
            StatusCode::NOT_FOUND,
            "Unavailable in this personal local-only fork",
        )
            .into_response();
    }
    // Local agents may use API keys for attribution. Historical human logins
    // and OAuth tokens must not reintroduce account switching.
    let credential = request
        .headers()
        .get(header::AUTHORIZATION)
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("Bearer "))
        })
        .or_else(|| {
            request
                .headers()
                .get(header::COOKIE)
                .and_then(|value| value.to_str().ok())
                .and_then(|cookies| {
                    cookies
                        .split(';')
                        .find_map(|cookie| cookie.trim().strip_prefix("lific_token="))
                })
                .map(Some)
        });
    if (path.starts_with("/api/") || path == "/mcp")
        && path != "/api/auth/auto-login"
        && let Some(token) = credential
    {
        let valid = (|| {
            let token = token?;
            let conn = pool.read().ok()?;
            let operator = db::queries::users::first_admin(&conn).ok()??;
            if token.starts_with("lific_sess_") {
                let user = db::queries::users::validate_session(&conn, token).ok()?;
                return (user.id == operator.id).then_some(());
            }
            // Only browser sessions can change tool credentials or profile.
            if path.starts_with("/api/auth/") || path == "/api/workspace" {
                return None;
            }
            drop(conn);
            let user = crate::auth::resolve_api_key_user(&pool, token).ok()?;
            match user {
                None => Some(()),
                Some(user) if user.id == operator.id => Some(()),
                Some(user) => {
                    let conn = pool.read().ok()?;
                    let owned: bool = conn
                        .query_row(
                            "SELECT is_bot = 1 AND owner_id = ?2 FROM users WHERE id = ?1",
                            rusqlite::params![user.id, operator.id],
                            |row| row.get(0),
                        )
                        .ok()?;
                    owned.then_some(())
                }
            }
        })()
        .is_some();
        if !valid {
            return (
                StatusCode::UNAUTHORIZED,
                "Refresh the local app to continue",
            )
                .into_response();
        }
    }
    // Project publication and ownership grants share ordinary CRUD routes.
    // Reject these fields before their handlers can change anything.
    if (request.method() == Method::PUT || request.method() == Method::POST)
        && (path == "/api/projects"
            || (path.starts_with("/api/projects/") && path.split('/').count() == 4))
    {
        let (parts, body) = request.into_parts();
        let bytes = match to_bytes(body, 2 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        };
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && (json.get("is_public").is_some_and(|value| !value.is_null())
                || json
                    .get("lead_user_id")
                    .is_some_and(|value| !value.is_null()))
        {
            return (StatusCode::BAD_REQUEST, "Projects belong to your local workspace; sharing and ownership transfers are unavailable").into_response();
        }
        request = Request::from_parts(parts, Body::from(bytes));
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn historical_credentials_cannot_select_a_different_person() {
        let pool = db::open_memory().unwrap();
        let (operator, historical, old_session) = {
            let conn = pool.write().unwrap();
            let operator = db::queries::users::create_passwordless_admin(&conn, "Owner").unwrap();
            conn.execute("INSERT INTO users (username, email, password_hash, display_name) VALUES ('old-person', 'old@local', 'unusable', 'Old author')", []).unwrap();
            let historical = conn.last_insert_rowid();
            let session = db::queries::users::create_session(&conn, historical, None).unwrap();
            (operator.id, historical, session.token)
        };
        let owner_key = crate::auth::create_api_key(&pool, "owner-tool", Some(operator)).unwrap();
        let old_key = crate::auth::create_api_key(&pool, "old-tool", Some(historical)).unwrap();
        prepare(&pool).unwrap();
        assert!(crate::auth::resolve_api_key_user(&pool, &owner_key).is_ok());
        assert!(crate::auth::resolve_api_key_user(&pool, &old_key).is_err());
        assert!(db::queries::users::get_user_by_id(&pool.read().unwrap(), historical).is_ok());
        assert!(db::queries::users::validate_session(&pool.read().unwrap(), &old_session).is_err());
        let app = axum::Router::new()
            .fallback(|| async { StatusCode::OK })
            .layer(axum::middleware::from_fn_with_state(pool, requests));
        for header in [header::AUTHORIZATION, header::COOKIE] {
            let value = if header == header::COOKIE {
                format!("lific_token={old_session}")
            } else {
                format!("Bearer {old_session}")
            };
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/projects")
                        .header(header, value)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        // An expired cookie must not stop the browser establishing its local session.
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/auto-login")
                    .header(header::COOKIE, format!("lific_token={old_session}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
