//! Integration tests for agent-exec serve sub-command.
//!
//! Starts the HTTP server in a background process, then makes real HTTP requests
//! to validate all endpoints.

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

mod support;

use support::{assert_common_fields, binary};

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
    root: tempfile::TempDir,
}

struct ServeProcessBuilder {
    token: Option<String>,
    allow_origin: Option<String>,
    extra_args: Vec<String>,
}

impl ServeProcessBuilder {
    fn new() -> Self {
        Self {
            token: None,
            allow_origin: None,
            extra_args: vec![],
        }
    }

    fn token(mut self, t: &str) -> Self {
        self.token = Some(t.to_string());
        self
    }

    fn allow_origin(mut self, origin: &str) -> Self {
        self.allow_origin = Some(origin.to_string());
        self
    }

    fn start(self) -> ServeProcess {
        // `free_port` closes its listener before `serve` binds, so two tests
        // running concurrently can be handed the same port and one of them exits
        // with "address already in use". Retry on a fresh port instead of
        // reporting that lost race as a failure of whatever the test asserts.
        for attempt in 0..5 {
            if let Some(server) = self.try_start() {
                return server;
            }
            thread::sleep(Duration::from_millis(50 * (attempt + 1)));
        }
        panic!("serve process did not start on any of 5 candidate ports");
    }

    /// Spawn one server and return it only once it answers `/health` itself.
    ///
    /// Returns `None` when the child lost the port race or never became
    /// reachable, so the caller can retry on a different port.
    fn try_start(&self) -> Option<ServeProcess> {
        let root = tempfile::tempdir().expect("create tempdir");
        let port = free_port();
        let bind = format!("127.0.0.1:{port}");

        let mut cmd = Command::new(binary());
        cmd.args(["serve", "--bind", &bind]);
        if let Some(ref origin) = self.allow_origin {
            cmd.args(["--allow-origin", origin]);
        }
        for arg in &self.extra_args {
            cmd.arg(arg);
        }
        cmd.env("AGENT_EXEC_ROOT", root.path());
        if let Some(ref token) = self.token {
            cmd.env("AGENT_EXEC_SERVE_TOKEN", token);
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::null());

        let mut child = cmd.spawn().expect("spawn serve process");

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            // A child that already exited lost the port race; a live child that
            // serves `/health` owns the port we are about to use.
            if matches!(child.try_wait(), Ok(Some(_))) {
                let _ = child.wait();
                return None;
            }
            if health_is_ready(port) {
                return Some(ServeProcess { child, port, root });
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

impl ServeProcess {
    fn start() -> Self {
        ServeProcessBuilder::new().start()
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn root_path(&self) -> &std::path::Path {
        self.root.path()
    }
}

impl Drop for ServeProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// True when `/health` on `port` answers with the canonical health envelope.
///
/// Stronger than a bare TCP connect: it proves the server we just spawned is the
/// one listening, not a leftover socket from a concurrently starting test.
fn health_is_ready(port: u16) -> bool {
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "2",
            &format!("http://127.0.0.1:{port}/health"),
        ])
        .output();
    let Ok(output) = output else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&output.stdout)
        .is_ok_and(|json| json["type"] == "health")
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
    post_json_with_auth(url, body, None)
}

fn post_json_with_auth(url: &str, body: &str, auth: Option<&str>) -> (u16, serde_json::Value) {
    let auth_header = auth.map(|token| format!("Authorization: Bearer {token}"));
    let mut args = vec![
        "-s".to_string(),
        "-w".to_string(),
        "\n%{http_code}".to_string(),
        "-X".to_string(),
        "POST".to_string(),
        "-H".to_string(),
        "Content-Type: application/json".to_string(),
    ];
    if let Some(ref h) = auth_header {
        args.push("-H".to_string());
        args.push(h.clone());
    }
    if !body.is_empty() {
        args.push("-d".to_string());
        args.push(body.to_string());
    }
    args.push(url.to_string());
    let output = Command::new("curl")
        .args(&args)
        .output()
        .expect("curl POST");
    parse_curl_output(&String::from_utf8_lossy(&output.stdout))
}

fn options_request(url: &str, origin: Option<&str>) -> (u16, Vec<(String, String)>) {
    let mut args = vec![
        "-s".to_string(),
        "-w".to_string(),
        "\n%{http_code}".to_string(),
        "-X".to_string(),
        "OPTIONS".to_string(),
        "-D".to_string(),
        "-".to_string(),
    ];
    if let Some(o) = origin {
        args.push("-H".to_string());
        args.push(format!("Origin: {o}"));
        args.push("-H".to_string());
        args.push("Access-Control-Request-Method: POST".to_string());
    }
    args.push(url.to_string());
    let output = Command::new("curl")
        .args(&args)
        .output()
        .expect("curl OPTIONS");
    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let status_line = raw.trim_end().lines().last().unwrap_or("0");
    let status: u16 = status_line.trim().parse().unwrap_or(0);
    let headers: Vec<(String, String)> = raw
        .lines()
        .filter(|l| l.contains(':'))
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.trim().to_lowercase(), v.trim().to_string()))
        })
        .collect();
    (status, headers)
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
    let job_id = json["job_id"].as_str().expect("missing job_id");
    assert_eq!(
        job_id.len(),
        32,
        "job_id must be fixed-length hex: {job_id}"
    );
    assert!(
        job_id
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "job_id must be lowercase hex: {job_id}"
    );
    assert_common_fields(&json);
}

#[test]
fn test_status_tail_wait_kill_accept_prefix_and_return_canonical_id() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(&srv.url("/exec"), r#"{"command":["sleep","60"]}"#);
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");
    let prefix = &job_id[..10];

    let (status_status, status_json) = get_json(&srv.url(&format!("/status/{prefix}")));
    assert_eq!(
        status_status, 200,
        "GET /status by prefix failed: {status_json}"
    );
    assert_eq!(status_json["job_id"].as_str().unwrap_or(""), job_id);

    let (tail_status, tail_json) = get_json(&srv.url(&format!("/tail/{prefix}")));
    assert_eq!(tail_status, 200, "GET /tail by prefix failed: {tail_json}");
    assert_eq!(tail_json["job_id"].as_str().unwrap_or(""), job_id);

    let (wait_status, wait_json) = get_json(&srv.url(&format!("/wait/{prefix}")));
    assert_eq!(wait_status, 200, "GET /wait by prefix failed: {wait_json}");
    assert_eq!(wait_json["job_id"].as_str().unwrap_or(""), job_id);

    let (kill_status, kill_json) = post_json(&srv.url(&format!("/kill/{prefix}")), r#"{}"#);
    assert_eq!(kill_status, 200, "POST /kill by prefix failed: {kill_json}");
    assert_eq!(kill_json["job_id"].as_str().unwrap_or(""), job_id);
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
    assert!(json.get("stdout").is_some(), "missing stdout in: {json}");
    assert_common_fields(&json);
}

#[test]
fn test_wait_returns_terminal_state() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sh","-c","printf 'waitme\\n'; printf 'waiterr\\n' >&2"]}"#,
    );
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    let (status, json) = get_json(&srv.url(&format!("/wait/{job_id}")));
    assert_eq!(status, 200, "GET /wait failed: {json}");
    assert!(json.get("state").is_some(), "missing state in: {json}");
    assert_eq!(json["stdout"].as_str(), Some("waitme\n"));
    assert_eq!(json["stderr"].as_str(), Some("waiterr\n"));
    assert_eq!(json["encoding"].as_str(), Some("utf-8-lossy"));
    assert_eq!(json["stdout_range"], serde_json::json!([0, 7]));
    assert_eq!(json["stderr_range"], serde_json::json!([0, 8]));
    assert_eq!(json["stdout_total_bytes"].as_u64(), Some(7));
    assert_eq!(json["stderr_total_bytes"].as_u64(), Some(8));
    assert_common_fields(&json);
    let state = json["state"].as_str().expect("state is string");
    assert!(
        matches!(state, "exited" | "killed" | "failed"),
        "expected terminal state after wait, got: {state}"
    );
}

#[test]
fn test_wait_returns_output_after_terminal_before_drain() {
    let srv = ServeProcess::start();
    let (_, exec_json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sh","-c","(sleep 1; printf 'late-http-stdout\\n'; printf 'late-http-stderr\\n' >&2) &"]}"#,
    );
    let job_id = exec_json["job_id"]
        .as_str()
        .expect("job_id in exec response");

    let (status, json) = get_json(&srv.url(&format!("/wait/{job_id}")));
    assert_eq!(status, 200, "GET /wait failed: {json}");
    assert_eq!(json["stdout"].as_str(), Some("late-http-stdout\n"));
    assert_eq!(json["stderr"].as_str(), Some("late-http-stderr\n"));
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
fn test_status_ambiguous_prefix_returns_400() {
    let srv = ServeProcess::start();
    let first = "01abcdef000000000000000000000001";
    let second = "01abcdef000000000000000000000002";
    std::fs::create_dir_all(srv.root_path().join(first)).expect("create first job dir");
    std::fs::create_dir_all(srv.root_path().join(second)).expect("create second job dir");

    let (status, json) = get_json(&srv.url("/status/01abcdef"));
    assert_eq!(status, 400, "expected 400 for ambiguous prefix: {json}");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "ambiguous_job_id");
    assert_common_fields(&json);

    let details = &json["error"]["details"];
    assert!(!details.is_null(), "error.details must be present: {json}");
    let candidates = details["candidates"]
        .as_array()
        .expect("details.candidates must be an array");
    assert_eq!(candidates.len(), 2, "expected 2 candidates: {json}");
    assert!(
        candidates.iter().any(|c| c.as_str() == Some(first)),
        "candidates must include first ID: {json}"
    );
    assert!(
        candidates.iter().any(|c| c.as_str() == Some(second)),
        "candidates must include second ID: {json}"
    );
    assert_eq!(
        details["truncated"].as_bool(),
        Some(false),
        "truncated must be false: {json}"
    );
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
fn test_exec_until_override() {
    let srv = ServeProcess::start();
    let start = std::time::Instant::now();
    let (status, json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sh","-c","exit 7"],"until":1}"#,
    );
    let elapsed = start.elapsed();
    assert_eq!(status, 200, "POST /exec with until=1 failed: {json}");
    assert_eq!(json["exit_code"], 7);
    assert!(
        elapsed < Duration::from_secs(5),
        "until=1 should return quickly, took {elapsed:?}"
    );
}

#[test]
fn test_exec_wait_false() {
    let srv = ServeProcess::start();
    let start = std::time::Instant::now();
    let (status, json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sleep","60"],"wait":false}"#,
    );
    let elapsed = start.elapsed();
    assert_eq!(status, 200, "POST /exec with wait=false failed: {json}");
    assert!(
        elapsed < Duration::from_secs(5),
        "wait=false should return immediately, took {elapsed:?}"
    );
    let stdout = json["stdout"].as_str().unwrap_or("");
    assert!(stdout.is_empty(), "stdout should be empty for wait=false");
}

#[test]
fn test_exec_max_bytes() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sh","-c","dd if=/dev/zero bs=4096 count=1 2>/dev/null | tr '\\0' 'A'"],"max_bytes":1024}"#,
    );
    assert_eq!(status, 200, "POST /exec with max_bytes failed: {json}");
    let stdout = json["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.len() <= 1024,
        "stdout should be at most 1024 bytes, got {}",
        stdout.len()
    );
}

#[test]
fn abandon_job_after_exec_legacy_timeout_error_is_actionable() {
    let srv = ServeProcess::start();
    for body in [
        r#"{"command":["echo","hi"],"timeout":1}"#,
        r#"{"command":["echo","hi"],"timeout_ms":1000}"#,
    ] {
        let (status, json) = post_json(&srv.url("/exec"), body);
        assert_eq!(status, 400, "expected 400 for {body}: {json}");
        assert_eq!(json["ok"], false, "{body}: {json}");
        assert_eq!(json["error"]["code"], "removed_option", "{body}: {json}");
        let message = json["error"]["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("abandon_job_after") && message.contains("until"),
            "{body}: migration error must name both replacements: {message}"
        );
        assert_common_fields(&json);
        assert_eq!(
            abandon_job_after_job_dirs(srv.root_path()),
            0,
            "{body} must not create a job"
        );
    }
}

/// Number of job directories under a serve process's isolated jobs root.
fn abandon_job_after_job_dirs(root: &std::path::Path) -> usize {
    std::fs::read_dir(root)
        .expect("read jobs root")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count()
}

#[test]
fn abandon_job_after_exec_rejects_unacknowledged_abandonment() {
    let srv = ServeProcess::start();
    for body in [
        r#"{"command":["sleep","60"],"abandon_job_after":1}"#,
        r#"{"command":["sleep","60"],"abandon_job_after":1,"acknowledge_result_loss":false}"#,
    ] {
        let (status, json) = post_json(&srv.url("/exec"), body);
        assert_eq!(status, 400, "expected 400 for {body}: {json}");
        assert_eq!(
            json["error"]["code"], "result_loss_not_acknowledged",
            "{body}: {json}"
        );
        let message = json["error"]["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("may permanently lose unfinished results")
                && message.contains("acknowledge_result_loss")
                && message.contains("until"),
            "{body}: response must warn and name both alternatives: {message}"
        );
        assert_common_fields(&json);
        assert_eq!(
            abandon_job_after_job_dirs(srv.root_path()),
            0,
            "{body} must not create a job"
        );
    }
}

#[test]
fn abandon_job_after_exec_terminates_an_acknowledged_workload() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(
        &srv.url("/exec"),
        r#"{"command":["sleep","60"],"abandon_job_after":1,"acknowledge_result_loss":true,"until":6}"#,
    );
    assert_eq!(status, 200, "acknowledged abandonment must launch: {json}");
    let job_id = json["job_id"].as_str().expect("job_id").to_string();

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let final_status = loop {
        let (code, body) = get_json(&srv.url(&format!("/status/{job_id}")));
        assert_eq!(code, 200, "status query failed: {body}");
        if body["state"] != "running" {
            break body;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "abandon_job_after never terminated the workload: {body}"
        );
        thread::sleep(Duration::from_millis(200));
    };
    assert_eq!(
        final_status["abandoned_by"], "abandon_job_after",
        "{final_status}"
    );
    assert_eq!(final_status["result_loss"], true, "{final_status}");
}

#[test]
fn abandon_job_after_exec_until_expiry_leaves_the_job_running() {
    let srv = ServeProcess::start();
    let (status, json) = post_json(&srv.url("/exec"), r#"{"command":["sleep","30"],"until":1}"#);
    assert_eq!(status, 200, "{json}");
    assert_eq!(
        json["state"], "running",
        "until expiry must return a non-terminal job: {json}"
    );
    let job_id = json["job_id"].as_str().expect("job_id").to_string();
    let (code, body) = get_json(&srv.url(&format!("/status/{job_id}")));
    assert_eq!(code, 200, "{body}");
    assert_eq!(
        body["state"], "running",
        "the service must not signal the workload: {body}"
    );
    assert!(
        body.get("abandoned_by").is_none(),
        "observation must never mark abandonment: {body}"
    );
    let (kill_code, kill_body) = post_json(&srv.url(&format!("/kill/{job_id}")), "");
    assert_eq!(kill_code, 200, "{kill_body}");
}

// ---- Security integration tests ----

#[test]
fn test_non_loopback_bind_without_insecure_is_rejected() {
    let root = tempfile::tempdir().expect("create tempdir");
    let port = free_port();
    let bind = format!("0.0.0.0:{port}");

    let output = Command::new(binary())
        .args(["serve", "--bind", &bind])
        .env("AGENT_EXEC_ROOT", root.path())
        .output()
        .expect("run serve");

    assert!(
        !output.status.success(),
        "expected non-zero exit for non-loopback without --insecure"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("serve_unsafe_bind"),
        "stdout should contain serve_unsafe_bind error: {stdout}"
    );
}

#[test]
fn test_non_loopback_bind_insecure_without_token_is_rejected() {
    let root = tempfile::tempdir().expect("create tempdir");
    let port = free_port();
    let bind = format!("0.0.0.0:{port}");

    let output = Command::new(binary())
        .args(["serve", "--bind", &bind, "--insecure"])
        .env("AGENT_EXEC_ROOT", root.path())
        .env_remove("AGENT_EXEC_SERVE_TOKEN")
        .output()
        .expect("run serve");

    assert!(
        !output.status.success(),
        "expected non-zero exit for non-loopback --insecure without token"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("serve_unsafe_bind"),
        "stdout should contain serve_unsafe_bind error: {stdout}"
    );
}

#[test]
fn test_auth_token_required_returns_401() {
    let srv = ServeProcessBuilder::new().token("testsecret").start();

    let (status, json) = post_json(&srv.url("/exec"), r#"{"command":["echo","hi"]}"#);
    assert_eq!(status, 401, "expected 401 without token: {json}");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "unauthorized");
}

#[test]
fn test_auth_token_accepted() {
    let srv = ServeProcessBuilder::new().token("testsecret").start();

    let (status, json) = post_json_with_auth(
        &srv.url("/exec"),
        r#"{"command":["echo","hi"]}"#,
        Some("testsecret"),
    );
    assert_eq!(status, 200, "expected 200 with valid token: {json}");
    assert_eq!(json["ok"], true);
}

#[test]
fn test_auth_wrong_token_returns_401() {
    let srv = ServeProcessBuilder::new().token("testsecret").start();

    let (status, json) = post_json_with_auth(
        &srv.url("/exec"),
        r#"{"command":["echo","hi"]}"#,
        Some("wrongtoken"),
    );
    assert_eq!(status, 401, "expected 401 with wrong token: {json}");
    assert_eq!(json["error"]["code"], "unauthorized");
}

#[test]
fn test_auth_readonly_endpoints_no_token_required() {
    let srv = ServeProcessBuilder::new().token("testsecret").start();

    let (status, _) = get_json(&srv.url("/health"));
    assert_eq!(status, 200, "GET /health should not require auth");
}

#[test]
fn test_cors_absent_by_default() {
    let srv = ServeProcess::start();
    let (_, headers) = options_request(&srv.url("/exec"), Some("https://evil.com"));
    let has_acao = headers
        .iter()
        .any(|(k, _)| k == "access-control-allow-origin");
    assert!(!has_acao, "CORS header should not be present by default");
}

#[test]
fn test_cors_with_allow_origin() {
    let srv = ServeProcessBuilder::new()
        .allow_origin("https://example.com")
        .start();

    let (_, headers) = options_request(&srv.url("/exec"), Some("https://example.com"));
    let acao = headers
        .iter()
        .find(|(k, _)| k == "access-control-allow-origin")
        .map(|(_, v)| v.as_str());
    assert_eq!(
        acao,
        Some("https://example.com"),
        "expected CORS header for allowed origin, got headers: {headers:?}"
    );
}
