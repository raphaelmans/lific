use std::process::Command;

fn invoke(config: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lific"))
        .arg("--config")
        .arg(config)
        .args(args)
        .env_remove("LIFIC_URL")
        .env_remove("LIFIC_TOKEN")
        .env_remove("LIFIC_API_KEY")
        .output()
        .expect("run local-only fork")
}

#[test]
fn remote_commands_fail_before_connecting() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("lific.toml");
    std::fs::write(&config, "[server]\nhost = \"127.0.0.1\"\n").unwrap();
    for args in [
        vec![
            "--backend",
            "http",
            "--url",
            "https://example.invalid",
            "project",
            "list",
        ],
        vec![
            "login",
            "--url",
            "https://example.invalid",
            "--non-interactive",
        ],
        vec!["mcp", "--remote", "--url", "https://example.invalid"],
        vec!["start", "--host", "0.0.0.0"],
    ] {
        let output = invoke(&config, &args);
        assert!(!output.status.success(), "{args:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Local-only policy"), "{args:?}: {error}");
    }
}

#[test]
fn remote_config_and_imports_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("lific.toml");
    std::fs::write(
        &config,
        "[server]\npublic_url = \"https://example.invalid\"\n",
    )
    .unwrap();
    let output = invoke(&config, &["doctor", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Local-only policy"));

    std::fs::write(&config, "[server]\nhost = \"127.0.0.1\"\n").unwrap();
    let output = invoke(
        &config,
        &[
            "import",
            "github",
            "--repo",
            "owner/repo",
            "--project",
            "TEST",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("External imports are disabled"));
}

#[test]
fn local_http_ignores_proxy_environment() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    // A proxy trap would receive credentials if no_proxy() were missing.
    let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
    proxy.set_nonblocking(true).unwrap();
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let task = std::thread::spawn(move || {
        server.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if let Ok((mut stream, _)) = server.accept() {
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = [0; 4096];
                let read = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..read]);
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer local-test-key")
                );
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]").unwrap();
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "local server was never contacted"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("lific.toml");
    std::fs::write(&config, "[server]\nhost = \"127.0.0.1\"\n").unwrap();
    let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_lific"))
        .arg("--config")
        .arg(config)
        .args([
            "--backend",
            "http",
            "--url",
            &format!("http://localhost:{}", address.port()),
            "--api-key",
            "local-test-key",
            "project",
            "list",
            "--json",
        ])
        .env("HTTP_PROXY", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("http_proxy", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env("NO_PROXY", "")
        .env("no_proxy", "")
        .output()
        .unwrap();
    task.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(proxy.accept().is_err(), "a local request reached the proxy");
}
