//! Integration tests for agent-exec serve sub-command.
//!
//! Starts the HTTP server in a background process, then makes real HTTP requests
//! to validate all endpoints.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Path to the compiled binary.
fn binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.push("agent-exec");
    if cfg!(windows) {
        p.set_extension("exe");
    }
    p
}

/// Find a free port on localhost.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind to find free port")
        .local_addr()
        .expect("local addr")
        .port()
}

/// A running serve process that cleans up on drop.
struct ServeProcess {
    child: Child,
    port: u16,
    _root: tempfile::TempDir,
}

impl ServeProcess {
    fn start() -> Self {
        let root = tempfile::tempdir().expect("create tempdir");
        let port = free_port();
        let bind = format!("127.0.0.1:{port}");

        let child = Command::new(binary())
            .args(["serve", "--bind", &bind])
            .env("AGENT_EXEC_ROOT", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn serve process");

        // Wait for the server to accept connections (up to 10 s).
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "server did not start within 10 seconds on port {port}"
            );
            thread::sleep(Duration::from_millis(50));
        }

        ServeProcess {
            child,
            port,
            _root: root,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for ServeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Parse the output of `curl -s -w "\n%{http_code}" ...`.
/// The last line is the numeric HTTP status; everything before is the body.
fn parse_curl_output(raw: &str) -> (u16, serde_json::Value) {
    let raw = raw.trim_end_matches('\n');
    let last_newline = raw.rfind('\n').expect("status code line missing");
    let status_str = raw[last_newline + 1..].trim();
    let body = &raw[..last_newline];
    let status: u16 = status_str.parse().expect("parse HTTP status code");
    let json: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("response is not valid JSON: {e}\nbody: {body}"));
    (status, json)
}

fn get_json(url: &str) -> (u16, serde_json::Value) {
    let output = Command::new("curl")
        .args(["-s", "-w", "\n%{http_code}", url])
        .output()
        .expect("curl GET");
    parse_curl_output(&String::from_utf8_lossy(&output.stdout))
}

fn post_json(url: &str, body: &str) -> (u16, serde_json::Value) {
    let mut args = vec![
        "-s",
        "-w",
        "\n%{http_code}",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
    ];
    if !body.is_empty() {
        args.extend_from_slice(&["-d", body]);
    }
    args.push(url);
    let output = Command::new("curl")
        .args(&args)
        .output()
        .expect("curl POST");
    parse_curl_output(&String::from_utf8_lossy(&output.stdout))
}

fn assert_common_fields(json: &serde_json::Value) {
    assert!(
        json.get("schema_version").is_some(),
        "missing schema_version in: {json}"
    );
    assert!(json.get("ok").is_some(), "missing ok in: {json}");
    assert!(json.get("type").is_some(), "missing type in: {json}");
}

// ---- Tests ----

#[test]
fn test_health() {
    let srv = ServeProcess::start();
    let (status, json) = get_json(&srv.url("/health"));
    assert_eq!(status, 200, "expected 200 from /health: {json}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["type"], "health");
    assert_common_fields(&json);
}

#[test]
fn test_exec_returns_job_id() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(&srv.url("/exec"), r#"{"command":["echo","hello"]}"#);
    assert_eq!(status, 200, "POST /exec failed: {json}");
    assert_eq!(json["ok"], true);
    assert!(json.get("job_id").is_some(), "missing job_id in: {json}");
    assert_common_fields(&json);
}

#[test]
fn test_status_returns_state() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(&srv.url("/exec"), r#"{"command":["echo","hello"]}"#);
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    let (status, json) = get_json(&srv.url(&format!("/status/{job_id}")));
    assert_eq!(status, 200, "GET /status failed: {json}");
    assert!(json.get("state").is_some(), "missing state in: {json}");
    assert_common_fields(&json);
}

#[test]
fn test_tail_returns_stdout() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(&srv.url("/exec"), r#"{"command":["echo","tailtest"]}"#);
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    let (wait_status, wait_json) = get_json(&srv.url(&format!("/wait/{job_id}")));
    assert_eq!(
        wait_status, 200,
        "GET /wait failed before /tail: {wait_json}"
    );

    let (status, json) = get_json(&srv.url(&format!("/tail/{job_id}")));
    assert_eq!(status, 200, "GET /tail failed: {json}");
    assert!(
        json.get("stdout_tail").is_some(),
        "missing stdout_tail in: {json}"
    );
    assert_common_fields(&json);
}

#[test]
fn test_wait_returns_terminal_state() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(&srv.url("/exec"), r#"{"command":["echo","waitme"]}"#);
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    let (status, json) = get_json(&srv.url(&format!("/wait/{job_id}")));
    assert_eq!(status, 200, "GET /wait failed: {json}");
    assert!(json.get("state").is_some(), "missing state in: {json}");
    assert_common_fields(&json);
    let state = json["state"].as_str().expect("state is string");
    assert!(
        matches!(state, "exited" | "killed" | "failed"),
        "expected terminal state after wait, got: {state}"
    );
}

#[test]
fn test_kill_returns_ok() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(&srv.url("/exec"), r#"{"command":["sleep","60"]}"#);
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    // Give the job a moment to start running.
    thread::sleep(Duration::from_millis(300));

    let (status, json) = post_json(&srv.url(&format!("/kill/{job_id}")), r#"{}"#);
    assert_eq!(status, 200, "POST /kill failed: {json}");
    assert_eq!(json["ok"], true);
    assert_common_fields(&json);
}

#[test]
fn test_status_not_found() {
    let srv = ServeProcess::start();
    let (status, json) = get_json(&srv.url("/status/nonexistent_id_xyz"));
    assert_eq!(status, 404, "expected 404 for nonexistent job: {json}");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "job_not_found");
    assert_common_fields(&json);
}

#[test]
fn test_exec_empty_body_returns_400() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(&srv.url("/exec"), "");
    assert_eq!(status, 400, "expected 400 for empty body: {json}");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "invalid_request");
    assert_common_fields(&json);
}

#[test]
fn test_exec_missing_command_returns_400() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(&srv.url("/exec"), r#"{"cwd":"/tmp"}"#);
    assert_eq!(
        status, 400,
        "expected 400 for missing command field: {json}"
    );
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "invalid_request");
    assert_common_fields(&json);
}

#[test]
fn test_exec_rejects_wait_field() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["echo","hi"],"wait":true}"#,
    );
    assert_eq!(status, 400, "expected 400 for unknown wait field: {json}");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "invalid_request");
    assert_common_fields(&json);
}
