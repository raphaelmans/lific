//! Process tests for the personal, login-free workspace. Every server uses temporary data.
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Longest we ever wait for the server to answer.
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);

/// Kills the server on drop, including while a test is unwinding.
struct ServerChild(Child);

impl Drop for ServerChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A free TCP port on loopback. Bound and released, so this races with any
/// other process that grabs it in between; nothing else in the suite listens.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind loopback")
        .local_addr()
        .expect("local addr")
        .port()
}

/// A config file naming `db_path`, with auth required (the default, restated
/// so the test asserts against something the file actually says).
fn write_config(dir: &Path, db_path: &Path, port: u16) -> PathBuf {
    let config_path = dir.join("lific.toml");
    let mut file = std::fs::File::create(&config_path).expect("create config");
    write!(
        file,
        "[server]\nhost = \"127.0.0.1\"\nport = {port}\n\n\
         [database]\npath = {db}\n\n\
         [backup]\nenabled = false\n\n\
         [auth]\nrequired = true\nallow_signup = false\n",
        db = toml_string(db_path)
    )
    .expect("write config");
    config_path
}

fn toml_string(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('\\', "\\\\"))
}

fn lific(config: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lific"));
    // Never let the developer's own instance, token, or URL leak in.
    for name in [
        "LIFIC_URL",
        "LIFIC_API_KEY",
        "LIFIC_TOKEN",
        "LIFIC_INIT_ADMIN_NAME",
        "LIFIC_INIT_ADMIN_PASSWORD",
        "RUST_LOG",
    ] {
        command.env_remove(name);
    }
    command.env("NO_COLOR", "1");
    command.arg("--config").arg(config);
    command
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build client")
}

enum Boot {
    Healthy,
    /// The child gave up on its own. Carries its stderr, so the caller can
    /// tell "the port was taken" from a real failure.
    Exited(String),
    TimedOut,
}

/// Poll `/api/health` until it answers 200, or give up. Never hangs, and
/// notices a child that died instead of waiting out the full timeout on a
/// process that will never answer.
fn wait_healthy(client: &reqwest::blocking::Client, base: &str, child: &mut Child) -> Boot {
    let deadline = Instant::now() + BOOT_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            let mut stderr = String::new();
            if let Some(pipe) = child.stderr.as_mut() {
                use std::io::Read;
                let _ = pipe.read_to_string(&mut stderr);
            }
            return Boot::Exited(stderr);
        }
        if let Ok(response) = client.get(format!("{base}/api/health")).send()
            && response.status().is_success()
        {
            return Boot::Healthy;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Boot::TimedOut
}

/// Start the server on a free port and wait for it to answer.
///
/// `free_port` releases the port before the server claims it, so another
/// process on the machine can take it in between. That is a flake, not a
/// failure: retry on a fresh port a couple of times before believing it.
fn boot_server(
    dir: &Path,
    db_path: &Path,
    client: &reqwest::blocking::Client,
) -> (ServerChild, u16) {
    let mut last = String::new();
    for attempt in 1..=3 {
        let port = free_port();
        let config = write_config(dir, db_path, port);
        let mut server = ServerChild(
            lific(&config)
                .args(["start"])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn server"),
        );
        match wait_healthy(client, &format!("http://127.0.0.1:{port}"), &mut server.0) {
            Boot::Healthy => return (server, port),
            Boot::Exited(stderr) if stderr.contains("in use") || stderr.contains("EADDRINUSE") => {
                last = format!("attempt {attempt}: port {port} was taken");
            }
            Boot::Exited(stderr) => panic!("server exited on attempt {attempt}: {stderr}"),
            Boot::TimedOut => panic!("no answer within {BOOT_TIMEOUT:?} on attempt {attempt}"),
        }
    }
    panic!("could not find a free port in 3 attempts ({last})");
}

#[test]
fn personal_init_and_restart_override_old_login_and_signup_settings() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let database = root.join("lific.db");
    let config = write_config(&root, &database, free_port());
    let output = lific(&config)
        .args(["init", "--no-service", "--name", "Personal test", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Simulate an old database with login/signup enabled and a second person's
    // session. Startup preserves historical identities but retires that session.
    {
        let conn = rusqlite::Connection::open(&database).unwrap();
        conn.execute(
            "UPDATE instance_settings SET allow_signup = 1, web_auto_login = 0",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO users (username, email, password_hash, display_name, is_admin, is_bot) SELECT 'historical', 'historical@local', password_hash, 'Historical author', 0, 0 FROM users LIMIT 1", []).unwrap();
        let historical = conn.last_insert_rowid();
        conn.execute("INSERT INTO sessions (token, user_id, expires_at) VALUES ('old-session-hash', ?1, datetime('now', '+1 day'))", [historical]).unwrap();
    }
    let client = client();
    let (_server, port) = boot_server(&root, &database, &client);
    let base = format!("http://127.0.0.1:{port}");
    let response = client
        .post(format!("{base}/api/auth/auto-login"))
        .send()
        .unwrap();
    assert!(response.status().is_success());
    let session: serde_json::Value = response.json().unwrap();
    assert_eq!(session["user"]["username"], "personal-test");
    let conn = rusqlite::Connection::open(&database).unwrap();
    let policy: (bool, bool) = conn
        .query_row(
            "SELECT allow_signup, web_auto_login FROM instance_settings",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(policy, (false, true));
    let historical: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM users WHERE username = 'historical'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(historical, 1, "keep authorship history");
    let old_sessions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE token = 'old-session-hash'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_sessions, 0);
    for path in [
        "/api/auth/login",
        "/api/auth/signup",
        "/api/users",
        "/oauth/register",
    ] {
        assert_eq!(
            client
                .post(format!("{base}{path}"))
                .send()
                .unwrap()
                .status()
                .as_u16(),
            404,
            "{path}"
        );
    }
    assert_eq!(
        client
            .get(format!("{base}/api/projects"))
            .bearer_auth("old-key")
            .send()
            .unwrap()
            .status()
            .as_u16(),
        401
    );
}

#[test]
fn start_needs_explicit_personal_initialization() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let database = root.join("missing.db");
    let config = write_config(&root, &database, free_port());
    let output = lific(&config).arg("start").output().unwrap();
    assert!(!output.status.success());
    assert!(!database.exists());
    let output = lific(&config)
        .args(["start", "--init-if-missing"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
    assert!(!database.exists());
}
