//! Integration tests for agent-exec v0.1 commands.
//!
//! Each test runs the compiled binary and validates that:
//!   1. stdout contains valid JSON only.
//!   2. The JSON contains `schema_version`, `ok`, and `type` fields.
//!   3. Command-specific fields are present.

use std::path::PathBuf;
use std::process::Command;

/// Path to the compiled binary.
fn binary() -> PathBuf {
    // Prefer the current exe's directory (works inside cargo test).
    let mut p = std::env::current_exe().expect("current exe");
    p.pop(); // remove test binary name
             // In release mode there's no "deps" subdirectory; try both.
    if p.ends_with("deps") {
        p.pop();
    }
    // Binary name is "agent-exec" as defined in [[bin]] of Cargo.toml.
    p.push("agent-exec");
    if cfg!(windows) {
        p.set_extension("exe");
    }
    p
}

fn run_cmd_with_root(args: &[&str], root: Option<&str>) -> serde_json::Value {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Stdout must be a single valid JSON object.
    assert!(
        !stdout.trim().is_empty(),
        "stdout is empty (stderr: {stderr})\nargs: {args:?}"
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("stdout is not valid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}")
    })
}

/// Validate the common envelope fields.
fn assert_envelope(v: &serde_json::Value, expected_type: &str, expected_ok: bool) {
    assert_eq!(
        v["schema_version"].as_str().unwrap_or(""),
        "0.1",
        "schema_version mismatch: {v}"
    );
    assert_eq!(
        v["ok"].as_bool().unwrap_or(!expected_ok),
        expected_ok,
        "ok mismatch: {v}"
    );
    assert_eq!(
        v["type"].as_str().unwrap_or(""),
        expected_type,
        "type mismatch: {v}"
    );
}

// ── run ────────────────────────────────────────────────────────────────────────

#[test]
fn run_returns_json_with_job_id() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["run", "echo", "hello"], Some(root));
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
    assert_eq!(v["state"].as_str().unwrap_or(""), "running");
}

#[test]
fn run_with_snapshot_after_includes_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    // Use snapshot_after=500ms; the echo command finishes quickly.
    let v = run_cmd_with_root(
        &["run", "--snapshot-after", "500", "echo", "snapshot_test"],
        Some(root),
    );
    assert_envelope(&v, "run", true);
    // snapshot field may or may not contain the output depending on timing,
    // but the field itself must be present.
    assert!(v.get("snapshot").is_some(), "snapshot field missing: {v}");
    let snapshot = &v["snapshot"];
    assert_eq!(snapshot["encoding"].as_str().unwrap_or(""), "utf-8-lossy");
    // Verify correct field names: stdout_tail / stderr_tail (not stdout / stderr).
    assert!(
        snapshot.get("stdout_tail").is_some(),
        "snapshot.stdout_tail missing: {snapshot}"
    );
    assert!(
        snapshot.get("stderr_tail").is_some(),
        "snapshot.stderr_tail missing: {snapshot}"
    );
}

// ── status ─────────────────────────────────────────────────────────────────────

#[test]
fn status_returns_json_for_existing_job() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    // First run a job.
    let run_v = run_cmd_with_root(&["run", "echo", "hi"], Some(root));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Then query status.
    let v = run_cmd_with_root(&["status", &job_id], Some(root));
    assert_envelope(&v, "status", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("state").is_some(), "state missing");
    assert!(v.get("started_at").is_some(), "started_at missing");
}

#[test]
fn status_error_for_unknown_job() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["status", "NONEXISTENT_JOB_ID_XYZ"], Some(root));
    assert_eq!(
        v["ok"].as_bool().unwrap_or(true),
        false,
        "expected ok=false for unknown job: {v}"
    );
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    // Verify the error code is "job_not_found", not "internal_error".
    assert_eq!(
        v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found",
        "expected error.code=job_not_found: {v}"
    );
}

#[test]
fn tail_error_for_unknown_job() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["tail", "NONEXISTENT_JOB_ID_XYZ"], Some(root));
    assert_eq!(v["ok"].as_bool().unwrap_or(true), false);
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

#[test]
fn kill_error_for_unknown_job() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["kill", "NONEXISTENT_JOB_ID_XYZ"], Some(root));
    assert_eq!(v["ok"].as_bool().unwrap_or(true), false);
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

#[test]
fn wait_error_for_unknown_job() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["wait", "NONEXISTENT_JOB_ID_XYZ"], Some(root));
    assert_eq!(v["ok"].as_bool().unwrap_or(true), false);
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

// ── tail ───────────────────────────────────────────────────────────────────────

#[test]
fn tail_returns_json_with_encoding() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    // Run and wait briefly for the echo to complete.
    let run_v = run_cmd_with_root(
        &["run", "--snapshot-after", "300", "echo", "tail_test"],
        Some(root),
    );
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let v = run_cmd_with_root(&["tail", &job_id], Some(root));
    assert_envelope(&v, "tail", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert_eq!(v["encoding"].as_str().unwrap_or(""), "utf-8-lossy");
    assert!(v.get("stdout").is_some(), "stdout missing");
    assert!(v.get("stderr").is_some(), "stderr missing");
}

// ── wait ───────────────────────────────────────────────────────────────────────

#[test]
fn wait_returns_json_after_job_finishes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    let run_v = run_cmd_with_root(&["run", "echo", "done"], Some(root));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait with timeout=5s; echo finishes fast.
    let v = run_cmd_with_root(&["wait", "--timeout-ms", "5000", &job_id], Some(root));
    assert_envelope(&v, "wait", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("state").is_some(), "state missing");
}

// ── kill ───────────────────────────────────────────────────────────────────────

#[test]
fn kill_returns_json() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    // Run a long-running command.
    let run_v = run_cmd_with_root(&["run", "sleep", "60"], Some(root));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Brief wait to let the supervisor start the child.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = run_cmd_with_root(&["kill", "--signal", "KILL", &job_id], Some(root));
    assert_envelope(&v, "kill", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("signal").is_some(), "signal missing");
}

// ── full.log ───────────────────────────────────────────────────────────────────

#[test]
fn run_creates_full_log() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    let run_v = run_cmd_with_root(
        &["run", "--snapshot-after", "400", "echo", "full_log_test"],
        Some(root),
    );
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // full.log must exist alongside stdout.log and stderr.log.
    let full_log = std::path::Path::new(root).join(&job_id).join("full.log");
    assert!(
        full_log.exists(),
        "full.log not found at {}",
        full_log.display()
    );
}

// ── schema_version sanity ──────────────────────────────────────────────────────

#[test]
fn all_commands_use_schema_version_0_1() {
    // Already verified individually above; this test documents the invariant.
    assert_eq!(agent_shell::schema::SCHEMA_VERSION, "0.1");
}

// ── contract v0.1: retryable field ─────────────────────────────────────────────

/// Spec requirement: error object MUST contain code, message, retryable.
#[test]
fn error_response_has_retryable_field() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let v = run_cmd_with_root(&["status", "NONEXISTENT_JOB_CONTRACT_TEST"], Some(root));
    let error = v.get("error").expect("error object missing");
    assert!(error.get("code").is_some(), "error.code missing: {error}");
    assert!(
        error.get("message").is_some(),
        "error.message missing: {error}"
    );
    assert!(
        error.get("retryable").is_some(),
        "error.retryable missing (required by spec): {error}"
    );
    // job_not_found is a permanent failure — retryable must be false.
    assert_eq!(
        error["retryable"].as_bool().unwrap_or(true),
        false,
        "job_not_found should have retryable=false: {error}"
    );
}

// ── contract v0.1: exit codes ──────────────────────────────────────────────────

/// Spec: expected failure (job not found) → exit code 1.
#[test]
fn status_unknown_job_exits_with_code_1() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", root)
        .args(["status", "NONEXISTENT_EXIT_CODE_TEST"])
        .output()
        .expect("run binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 for unknown job"
    );
}

/// Spec: CLI usage error → exit code 2.
#[test]
fn invalid_subcommand_exits_with_code_2() {
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["__no_such_subcommand__"])
        .output()
        .expect("run binary");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for invalid subcommand"
    );
}

// ── contract v0.1: run -- <cmd> separator ──────────────────────────────────────

/// Spec: `agent-exec run [options] -- <cmd> [args...]` form MUST be accepted.
#[test]
fn run_with_double_dash_separator() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    // Use `--` before the command as the spec requires.
    let v = run_cmd_with_root(&["run", "--", "echo", "hello_dash"], Some(root));
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
}

// ── contract v0.1: stdout JSON-only ────────────────────────────────────────────

/// Spec: stdout MUST contain a single JSON object only (no extra lines or text).
#[test]
fn stdout_is_single_json_object() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", root)
        .args(["status", "NONEXISTENT_STDOUT_JSON_TEST"])
        .output()
        .expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    // Exactly one non-empty line on stdout.
    assert_eq!(
        lines.len(),
        1,
        "stdout should contain exactly 1 line (JSON), got {}: {:?}",
        lines.len(),
        lines
    );
    // That line must parse as a JSON object.
    let parsed: serde_json::Value =
        serde_json::from_str(lines[0]).expect("stdout line is not valid JSON");
    assert!(parsed.is_object(), "stdout JSON is not an object: {parsed}");
}

/// Spec: stderr is used only for diagnostic logs (not JSON output).
#[test]
fn stderr_contains_no_json_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", root)
        .env("RUST_LOG", "info")
        .args(["status", "NONEXISTENT_STDERR_TEST"])
        .output()
        .expect("run binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // stderr must not start with '{' (no JSON envelope leaking to stderr).
    for line in stderr.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            assert!(
                !trimmed.starts_with('{'),
                "stderr contains JSON-like output (should be logs only): {trimmed}"
            );
        }
    }
}
