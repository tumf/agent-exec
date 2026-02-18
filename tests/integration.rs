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
    // Binary name matches package name in Cargo.toml.
    p.push("agent-shell");
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

// ── log files exist immediately after run ──────────────────────────────────────

#[test]
fn run_creates_all_log_files_immediately() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    // Run WITHOUT snapshot-after so the test doesn't wait for the child.
    let run_v = run_cmd_with_root(&["run", "echo", "log_files_test"], Some(root));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let job_path = std::path::Path::new(root).join(&job_id);

    // All three log files must exist immediately after `run` returns,
    // even before the supervisor has written any content.
    for log_file in &["stdout.log", "stderr.log", "full.log"] {
        let p = job_path.join(log_file);
        assert!(
            p.exists(),
            "{log_file} not found at {} immediately after run",
            p.display()
        );
    }
}

// ── state.json null fields ─────────────────────────────────────────────────────

#[test]
fn state_json_required_fields_present_with_null_for_options() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();

    // Run a job and read back state.json from the job directory.
    let run_v = run_cmd_with_root(&["run", "echo", "state_test"], Some(root));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let state_path = std::path::Path::new(root).join(&job_id).join("state.json");
    assert!(state_path.exists(), "state.json not found");

    let raw = std::fs::read_to_string(&state_path).unwrap();
    let state: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // Required fields from spec: nested structure with job.id, job.status, job.started_at,
    // result.exit_code, result.signal, result.duration_ms, and top-level updated_at.
    let job = state.get("job").expect("job block missing from state.json");
    assert!(job.get("id").is_some(), "job.id missing from state.json");
    assert!(
        job.get("status").is_some(),
        "job.status missing from state.json"
    );
    assert!(
        job.get("started_at").is_some(),
        "job.started_at missing from state.json"
    );

    let result = state
        .get("result")
        .expect("result block missing from state.json");
    assert!(
        result.get("exit_code").is_some(),
        "result.exit_code missing from state.json (must be null)"
    );
    assert!(
        result.get("signal").is_some(),
        "result.signal missing from state.json (must be null)"
    );
    assert!(
        result.get("duration_ms").is_some(),
        "result.duration_ms missing from state.json (must be null)"
    );
    assert!(
        state.get("updated_at").is_some(),
        "updated_at missing from state.json"
    );

    // While the job was just spawned, these should be null (running).
    // (They may already be set if the echo finished before this read; that's fine.)
    // What we verify is that the keys are always present regardless.
    let exit_code = &result["exit_code"];
    let signal = &result["signal"];
    let duration_ms = &result["duration_ms"];

    // They must be either null or a concrete value, never absent.
    assert!(
        exit_code.is_null() || exit_code.is_number(),
        "result.exit_code must be null or number, got {exit_code}"
    );
    assert!(
        signal.is_null() || signal.is_string(),
        "result.signal must be null or string, got {signal}"
    );
    assert!(
        duration_ms.is_null() || duration_ms.is_number(),
        "result.duration_ms must be null or number, got {duration_ms}"
    );
}

// ── schema_version sanity ──────────────────────────────────────────────────────

#[test]
fn all_commands_use_schema_version_0_1() {
    // Already verified individually above; this test documents the invariant.
    assert_eq!(agent_shell::schema::SCHEMA_VERSION, "0.1");
}
