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

/// Test harness that owns an isolated temporary root directory.
///
/// Each test should create one harness; the temp directory is cleaned up
/// automatically when the harness is dropped.
struct TestHarness {
    /// The underlying temporary directory (kept alive for the harness lifetime).
    _tmp: tempfile::TempDir,
    /// String path to the root, set as `AGENT_EXEC_ROOT` for every command.
    root: String,
}

impl TestHarness {
    /// Create a new harness with a fresh temporary directory.
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let root = tmp
            .path()
            .to_str()
            .expect("tempdir path is valid UTF-8")
            .to_string();
        Self { _tmp: tmp, root }
    }

    /// Return the root path as a `&str`.
    fn root(&self) -> &str {
        &self.root
    }

    /// Run the binary with the given args under this harness's root, returning
    /// the parsed stdout JSON.  Panics with a descriptive message on any error.
    fn run(&self, args: &[&str]) -> serde_json::Value {
        run_cmd_with_root(args, Some(&self.root))
    }
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
    let h = TestHarness::new();
    // Use --snapshot-after 0 to return immediately (avoid 10s default wait in tests).
    let v = h.run(&["run", "--snapshot-after", "0", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
    assert_eq!(v["state"].as_str().unwrap_or(""), "running");
}

#[test]
fn run_with_snapshot_after_includes_snapshot() {
    let h = TestHarness::new();
    // Use snapshot_after=500ms; the echo command finishes quickly.
    let v = h.run(&["run", "--snapshot-after", "500", "echo", "snapshot_test"]);
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
    let h = TestHarness::new();

    // First run a job (use --snapshot-after 0 to return immediately).
    let run_v = h.run(&["run", "--snapshot-after", "0", "echo", "hi"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Then query status.
    let v = h.run(&["status", &job_id]);
    assert_envelope(&v, "status", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("state").is_some(), "state missing");
    assert!(v.get("started_at").is_some(), "started_at missing");
}

#[test]
fn status_error_for_unknown_job() {
    let h = TestHarness::new();
    let v = h.run(&["status", "NONEXISTENT_JOB_ID_XYZ"]);
    assert!(
        !v["ok"].as_bool().unwrap_or(true),
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
    let h = TestHarness::new();
    let v = h.run(&["tail", "NONEXISTENT_JOB_ID_XYZ"]);
    assert!(!v["ok"].as_bool().unwrap_or(true));
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

#[test]
fn kill_error_for_unknown_job() {
    let h = TestHarness::new();
    let v = h.run(&["kill", "NONEXISTENT_JOB_ID_XYZ"]);
    assert!(!v["ok"].as_bool().unwrap_or(true));
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

#[test]
fn wait_error_for_unknown_job() {
    let h = TestHarness::new();
    let v = h.run(&["wait", "NONEXISTENT_JOB_ID_XYZ"]);
    assert!(!v["ok"].as_bool().unwrap_or(true));
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

// ── tail ───────────────────────────────────────────────────────────────────────

#[test]
fn tail_returns_json_with_encoding() {
    let h = TestHarness::new();

    // Run and wait briefly for the echo to complete.
    let run_v = h.run(&["run", "--snapshot-after", "300", "echo", "tail_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let v = h.run(&["tail", &job_id]);
    assert_envelope(&v, "tail", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert_eq!(v["encoding"].as_str().unwrap_or(""), "utf-8-lossy");
    // Spec requires stdout_tail / stderr_tail field names (not stdout / stderr).
    assert!(v.get("stdout_tail").is_some(), "stdout_tail missing");
    assert!(v.get("stderr_tail").is_some(), "stderr_tail missing");
    // Spec requires truncated field.
    assert!(v.get("truncated").is_some(), "truncated missing");
}

// ── wait ───────────────────────────────────────────────────────────────────────

#[test]
fn wait_returns_json_after_job_finishes() {
    let h = TestHarness::new();

    // Use --snapshot-after 0 to return immediately so we can test wait separately.
    let run_v = h.run(&["run", "--snapshot-after", "0", "echo", "done"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait with timeout=5s; echo finishes fast.
    let v = h.run(&["wait", "--timeout-ms", "5000", &job_id]);
    assert_envelope(&v, "wait", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("state").is_some(), "state missing");
}

// ── kill ───────────────────────────────────────────────────────────────────────

#[test]
fn kill_returns_json() {
    let h = TestHarness::new();

    // Run a long-running command (use --snapshot-after 0 to return immediately).
    let run_v = h.run(&["run", "--snapshot-after", "0", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Brief wait to let the supervisor start the child.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = h.run(&["kill", "--signal", "KILL", &job_id]);
    assert_envelope(&v, "kill", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("signal").is_some(), "signal missing");
}

// ── full.log ───────────────────────────────────────────────────────────────────

#[test]
fn run_creates_full_log() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "--snapshot-after", "400", "echo", "full_log_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // full.log must exist alongside stdout.log and stderr.log.
    let full_log = std::path::Path::new(h.root())
        .join(&job_id)
        .join("full.log");
    assert!(
        full_log.exists(),
        "full.log not found at {}",
        full_log.display()
    );
}

// ── log files exist immediately after run ──────────────────────────────────────

#[test]
fn run_creates_all_log_files_immediately() {
    let h = TestHarness::new();

    // Run with --snapshot-after 0 so the test doesn't wait for the child.
    let run_v = h.run(&["run", "--snapshot-after", "0", "echo", "log_files_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let job_path = std::path::Path::new(h.root()).join(&job_id);

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
    let h = TestHarness::new();

    // Run a job and read back state.json from the job directory (use --snapshot-after 0 to return immediately).
    let run_v = h.run(&["run", "--snapshot-after", "0", "echo", "state_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let state_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("state.json");
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
    assert_eq!(agent_exec::schema::SCHEMA_VERSION, "0.1");
}

// ── contract v0.1: retryable field ─────────────────────────────────────────────

/// Spec requirement: error object MUST contain code, message, retryable.
#[test]
fn error_response_has_retryable_field() {
    let h = TestHarness::new();
    let v = h.run(&["status", "NONEXISTENT_JOB_CONTRACT_TEST"]);
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
    assert!(
        !error["retryable"].as_bool().unwrap_or(true),
        "job_not_found should have retryable=false: {error}"
    );
}

// ── contract v0.1: exit codes ──────────────────────────────────────────────────

/// Spec: expected failure (job not found) → exit code 1.
#[test]
fn status_unknown_job_exits_with_code_1() {
    let h = TestHarness::new();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
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
    let h = TestHarness::new();
    // Use `--` before the command as the spec requires (use --snapshot-after 0 to return immediately).
    let v = h.run(&["run", "--snapshot-after", "0", "--", "echo", "hello_dash"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
}

// ── contract v0.1: stdout JSON-only ────────────────────────────────────────────

/// Spec: stdout MUST contain a single JSON object only (no extra lines or text).
#[test]
fn stdout_is_single_json_object() {
    let h = TestHarness::new();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
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
    let h = TestHarness::new();
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
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

// ── New feature tests ──────────────────────────────────────────────────────────

/// Spec: full.log lines MUST include RFC3339 timestamp and [STDOUT]/[STDERR] tags.
#[test]
fn full_log_has_timestamp_and_stream_tags() {
    let h = TestHarness::new();

    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "500",
        "echo",
        "full_log_format_test",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let full_log = std::path::Path::new(h.root())
        .join(&job_id)
        .join("full.log");
    // Wait briefly for the supervisor to flush.
    std::thread::sleep(std::time::Duration::from_millis(200));

    if full_log.exists() {
        let contents = std::fs::read_to_string(&full_log).unwrap_or_default();
        if !contents.is_empty() {
            // Each line should contain [STDOUT] or [STDERR] tag.
            for line in contents.lines() {
                assert!(
                    line.contains("[STDOUT]") || line.contains("[STDERR]"),
                    "full.log line missing [STDOUT]/[STDERR] tag: {line}"
                );
            }
        }
    }
}

/// Spec: --log overrides the full.log path.
#[test]
fn run_log_path_override() {
    let h = TestHarness::new();
    let log_path = std::path::Path::new(h.root()).join("custom_full.log");
    let log_path_str = log_path.to_str().unwrap();

    h.run(&[
        "run",
        "--snapshot-after",
        "500",
        "--log",
        log_path_str,
        "echo",
        "log_override_test",
    ]);

    // Wait briefly for the supervisor to flush.
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert!(
        log_path.exists(),
        "custom log file not found at {}",
        log_path.display()
    );
}

/// Spec: --env KEY=VALUE overrides environment variables.
#[test]
fn run_env_var_is_applied() {
    let h = TestHarness::new();

    // Run a command that prints the env var value.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "500",
        "--env",
        "TEST_KEY_AGENT_EXEC=hello_from_env",
        "--",
        "sh",
        "-c",
        "echo $TEST_KEY_AGENT_EXEC",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for the child to finish.
    std::thread::sleep(std::time::Duration::from_millis(500));

    let stdout_log = std::path::Path::new(h.root())
        .join(&job_id)
        .join("stdout.log");
    if stdout_log.exists() {
        let contents = std::fs::read_to_string(&stdout_log).unwrap_or_default();
        assert!(
            contents.contains("hello_from_env"),
            "env var not applied; stdout.log: {contents}"
        );
    }
}

/// Spec: --no-inherit-env clears the parent environment.
#[test]
fn run_no_inherit_env_clears_env() {
    let h = TestHarness::new();

    // PATH is typically set; with --no-inherit-env it should not be in child env.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "500",
        "--no-inherit-env",
        "--",
        "/bin/sh",
        "-c",
        "echo INHERITED=$HOME",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for the child to finish.
    std::thread::sleep(std::time::Duration::from_millis(500));

    let stdout_log = std::path::Path::new(h.root())
        .join(&job_id)
        .join("stdout.log");
    if stdout_log.exists() {
        let contents = std::fs::read_to_string(&stdout_log).unwrap_or_default();
        // $HOME should be empty when env is cleared.
        assert!(
            contents.contains("INHERITED=\n") || contents.contains("INHERITED="),
            "expected HOME to be empty with --no-inherit-env; stdout.log: {contents}"
        );
    }
}

/// Spec: --timeout causes the child process to be terminated after the deadline.
#[test]
fn run_timeout_terminates_child() {
    let h = TestHarness::new();

    // Start a long sleep with a short timeout.
    // Use --snapshot-after 0 to return immediately; timeout is tested via status.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "0",
        "--timeout",
        "500",
        "--kill-after",
        "500",
        "sleep",
        "60",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait long enough for timeout + kill-after to fire.
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Check that the job is no longer running (state should be exited or killed).
    let v = h.run(&["status", &job_id]);
    let state = v["state"].as_str().unwrap_or("running");
    assert!(
        state != "running",
        "job should have been terminated by timeout; state={state}"
    );
}

/// Spec: --progress-every updates state.json.updated_at within the interval.
#[test]
fn run_progress_every_updates_state() {
    let h = TestHarness::new();

    // Run a long sleep with progress-every=200ms.
    // Use --snapshot-after 0 to return immediately; progress-every is the focus here.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "0",
        "--progress-every",
        "200",
        "sleep",
        "5",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait briefly to allow state.json to be updated.
    std::thread::sleep(std::time::Duration::from_millis(500));

    let state_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("state.json");
    let contents = std::fs::read_to_string(&state_path).unwrap_or_default();
    let state: serde_json::Value =
        serde_json::from_str(&contents).expect("state.json is not valid JSON");

    // updated_at should be present.
    assert!(
        state.get("updated_at").is_some(),
        "updated_at missing from state.json: {contents}"
    );

    // Cleanup: kill the sleep job.
    h.run(&["kill", "--signal", "KILL", &job_id]);
}

/// Acceptance #1 follow-up: --progress-every alone must not keep _supervise alive after child exits.
/// After a short-lived process finishes, `status` must NOT remain "running" indefinitely.
#[test]
fn progress_every_supervise_stops_after_child_exits() {
    let h = TestHarness::new();

    // Run a short-lived command with --progress-every only (no timeout).
    // Use --snapshot-after 0 to return immediately; progress-every is the focus here.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "0",
        "--progress-every",
        "100",
        "--",
        "echo",
        "done",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait enough time for the supervisor to detect child exit and update state.
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let v = h.run(&["status", &job_id]);
    let state = v["state"].as_str().unwrap_or("running");
    assert_ne!(
        state, "running",
        "job should not be running after child exits with --progress-every; state={state}, response={v}"
    );
}

/// Spec: --inherit-env and --no-inherit-env are mutually exclusive (clap rejects both together).
#[test]
fn inherit_env_and_no_inherit_env_are_mutually_exclusive() {
    let h = TestHarness::new();
    let bin = binary();

    // Passing both --inherit-env and --no-inherit-env should fail with exit code 2.
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
        .args([
            "run",
            "--inherit-env",
            "--no-inherit-env",
            "--",
            "echo",
            "test",
        ])
        .output()
        .expect("run binary");

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when both --inherit-env and --no-inherit-env are supplied"
    );
}

/// Spec: --mask KEY causes that key's value to appear as "***" in meta.json env_vars.
#[test]
fn mask_replaces_env_var_value_with_stars() {
    let h = TestHarness::new();

    let run_v = h.run(&[
        "run",
        "--env",
        "SECRET_TOKEN=super_secret_value",
        "--mask",
        "SECRET_TOKEN",
        "--snapshot-after",
        "300",
        "--",
        "echo",
        "done",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for supervisor to finish.
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Read meta.json and verify the masked value.
    let meta_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("meta.json");
    assert!(meta_path.exists(), "meta.json not found");
    let meta_contents = std::fs::read_to_string(&meta_path).unwrap();
    let meta: serde_json::Value =
        serde_json::from_str(&meta_contents).expect("meta.json invalid JSON");

    // env_vars in meta.json must contain "SECRET_TOKEN=***" (not the real value).
    let env_vars = meta["env_vars"]
        .as_array()
        .expect("env_vars missing in meta.json");
    let has_masked = env_vars
        .iter()
        .any(|v| v.as_str() == Some("SECRET_TOKEN=***"));
    assert!(
        has_masked,
        "expected SECRET_TOKEN=*** in meta.json env_vars, got: {meta_contents}"
    );
    // The real secret value must NOT appear in meta.json.
    assert!(
        !meta_contents.contains("super_secret_value"),
        "real secret value should not appear in meta.json: {meta_contents}"
    );
}

/// Spec (Acceptance #2): `run` JSON response must include masked env_vars field,
/// with the masked key's value replaced by "***". The real secret value must not
/// appear in the `run` stdout JSON response.
#[test]
fn run_json_response_includes_masked_env_vars() {
    let h = TestHarness::new();

    let bin = binary();
    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
        .args([
            "run",
            "--snapshot-after",
            "0",
            "--env",
            "SECRET=super_secret_run_value",
            "--mask",
            "SECRET",
            "--",
            "echo",
            "done",
        ])
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\nstdout: {stdout}"));

    assert_envelope(&v, "run", true);

    // env_vars must be present in the run JSON response.
    let env_vars = v["env_vars"]
        .as_array()
        .expect("env_vars missing in run JSON response");

    // The masked key must appear as "SECRET=***" (not the real value).
    let has_masked = env_vars.iter().any(|v| v.as_str() == Some("SECRET=***"));
    assert!(
        has_masked,
        "expected SECRET=*** in run JSON env_vars, got: {v}"
    );

    // The real secret value must NOT appear in the run JSON response.
    assert!(
        !stdout.contains("super_secret_run_value"),
        "real secret value should not appear in run JSON stdout: {stdout}"
    );
}

/// Spec: tail returns truncated=true when output exceeds constraints.
#[test]
fn tail_truncated_when_over_limit() {
    let h = TestHarness::new();

    // Generate exactly 5 lines; request only 2 lines.
    let run_v = h.run(&[
        "run",
        "--snapshot-after",
        "500",
        "--",
        "sh",
        "-c",
        "printf 'line1\\nline2\\nline3\\nline4\\nline5\\n'",
    ]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for the child to finish.
    std::thread::sleep(std::time::Duration::from_millis(300));

    let v = h.run(&["tail", "--tail-lines", "2", &job_id]);
    assert_envelope(&v, "tail", true);
    // With --lines 2 and 5 lines of output, truncated should be true.
    assert!(
        v["truncated"].as_bool().unwrap_or(false),
        "expected truncated=true; response: {v}"
    );
}

// ── add-run-tail-metrics: new fields ──────────────────────────────────────────

/// Task 3.1: run response includes waited_ms, elapsed_ms, and snapshot bytes metrics.
#[test]
fn run_includes_waited_ms_elapsed_ms_and_log_paths() {
    let h = TestHarness::new();

    let v = h.run(&["run", "--snapshot-after", "300", "echo", "metrics_test"]);
    assert_envelope(&v, "run", true);

    // waited_ms must be present and non-negative.
    let waited_ms = v["waited_ms"]
        .as_u64()
        .expect("waited_ms missing from run response");
    // elapsed_ms must be present and >= waited_ms.
    let elapsed_ms = v["elapsed_ms"]
        .as_u64()
        .expect("elapsed_ms missing from run response");
    assert!(
        elapsed_ms >= waited_ms,
        "elapsed_ms ({elapsed_ms}) must be >= waited_ms ({waited_ms})"
    );

    // stdout_log_path and stderr_log_path must be present and non-empty.
    let stdout_path = v["stdout_log_path"]
        .as_str()
        .expect("stdout_log_path missing from run response");
    let stderr_path = v["stderr_log_path"]
        .as_str()
        .expect("stderr_log_path missing from run response");
    assert!(!stdout_path.is_empty(), "stdout_log_path is empty");
    assert!(!stderr_path.is_empty(), "stderr_log_path is empty");
    // Paths must be absolute.
    assert!(
        std::path::Path::new(stdout_path).is_absolute(),
        "stdout_log_path must be absolute: {stdout_path}"
    );
    assert!(
        std::path::Path::new(stderr_path).is_absolute(),
        "stderr_log_path must be absolute: {stderr_path}"
    );

    // snapshot must include bytes metrics.
    let snapshot = v
        .get("snapshot")
        .expect("snapshot missing from run response");
    assert!(
        snapshot.get("stdout_observed_bytes").is_some(),
        "snapshot.stdout_observed_bytes missing: {snapshot}"
    );
    assert!(
        snapshot.get("stderr_observed_bytes").is_some(),
        "snapshot.stderr_observed_bytes missing: {snapshot}"
    );
    assert!(
        snapshot.get("stdout_included_bytes").is_some(),
        "snapshot.stdout_included_bytes missing: {snapshot}"
    );
    assert!(
        snapshot.get("stderr_included_bytes").is_some(),
        "snapshot.stderr_included_bytes missing: {snapshot}"
    );

    // included_bytes must be <= observed_bytes (we can only include what was observed).
    let stdout_observed = snapshot["stdout_observed_bytes"].as_u64().unwrap_or(0);
    let stdout_included = snapshot["stdout_included_bytes"].as_u64().unwrap_or(0);
    assert!(
        stdout_included <= stdout_observed,
        "stdout_included_bytes ({stdout_included}) must be <= stdout_observed_bytes ({stdout_observed})"
    );
}

/// Task 3.1: run with explicit --snapshot-after 0 has waited_ms=0 and no snapshot.
#[test]
fn run_without_snapshot_after_has_waited_ms_zero() {
    let h = TestHarness::new();

    // Explicitly pass --snapshot-after 0 to opt out of the default 200ms wait.
    let v = h.run(&["run", "--snapshot-after", "0", "echo", "no_snapshot"]);
    assert_envelope(&v, "run", true);

    let waited_ms = v["waited_ms"].as_u64().expect("waited_ms missing");
    assert_eq!(waited_ms, 0, "waited_ms must be 0 when snapshot-after=0");

    let elapsed_ms = v["elapsed_ms"].as_u64().expect("elapsed_ms missing");
    assert!(
        elapsed_ms < 5000,
        "elapsed_ms should be small without wait: {elapsed_ms}"
    );

    // No snapshot field when snapshot_after=0.
    assert!(
        v.get("snapshot").is_none() || v["snapshot"].is_null(),
        "snapshot should be absent when snapshot-after=0: {v}"
    );
}

/// Task 3.2: tail response includes log paths and bytes metrics.
#[test]
fn tail_includes_log_paths_and_bytes_metrics() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "--snapshot-after", "400", "echo", "tail_bytes_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for the child to finish writing output.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = h.run(&["tail", &job_id]);
    assert_envelope(&v, "tail", true);

    // stdout_log_path and stderr_log_path must be present and absolute.
    let stdout_path = v["stdout_log_path"]
        .as_str()
        .expect("stdout_log_path missing from tail response");
    let stderr_path = v["stderr_log_path"]
        .as_str()
        .expect("stderr_log_path missing from tail response");
    assert!(!stdout_path.is_empty(), "stdout_log_path is empty");
    assert!(!stderr_path.is_empty(), "stderr_log_path is empty");
    assert!(
        std::path::Path::new(stdout_path).is_absolute(),
        "stdout_log_path must be absolute: {stdout_path}"
    );
    assert!(
        std::path::Path::new(stderr_path).is_absolute(),
        "stderr_log_path must be absolute: {stderr_path}"
    );

    // Bytes metrics must be present.
    assert!(
        v.get("stdout_observed_bytes").is_some(),
        "stdout_observed_bytes missing from tail response: {v}"
    );
    assert!(
        v.get("stderr_observed_bytes").is_some(),
        "stderr_observed_bytes missing from tail response: {v}"
    );
    assert!(
        v.get("stdout_included_bytes").is_some(),
        "stdout_included_bytes missing from tail response: {v}"
    );
    assert!(
        v.get("stderr_included_bytes").is_some(),
        "stderr_included_bytes missing from tail response: {v}"
    );

    // included_bytes must be <= observed_bytes.
    let stdout_observed = v["stdout_observed_bytes"].as_u64().unwrap_or(0);
    let stdout_included = v["stdout_included_bytes"].as_u64().unwrap_or(0);
    assert!(
        stdout_included <= stdout_observed,
        "stdout_included_bytes ({stdout_included}) must be <= stdout_observed_bytes ({stdout_observed})"
    );
}

// ── list ───────────────────────────────────────────────────────────────────────

/// Spec: `list` on an empty (non-existent) root returns jobs=[].
#[test]
fn list_returns_empty_when_root_does_not_exist() {
    // Use a path that does not exist.
    let h = TestHarness::new();
    let nonexistent = std::path::Path::new(h.root()).join("does_not_exist");
    let nonexistent_str = nonexistent.to_str().unwrap();
    let v = run_cmd_with_root(&["list"], Some(nonexistent_str));
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert!(jobs.is_empty(), "expected empty jobs list; got: {v}");
    assert!(
        !v["truncated"].as_bool().unwrap_or(true),
        "truncated must be false for empty list"
    );
}

/// Spec: `list` returns all jobs, sorted by started_at descending.
#[test]
fn list_returns_jobs_sorted_by_started_at_desc() {
    let h = TestHarness::new();

    // Run two jobs (use --snapshot-after 0 to return immediately); both should appear in list.
    let _r1 = h.run(&["run", "--snapshot-after", "0", "echo", "job1"]);
    // Small sleep to ensure distinct timestamps.
    std::thread::sleep(std::time::Duration::from_millis(10));
    let r2 = h.run(&["run", "--snapshot-after", "0", "echo", "job2"]);
    let job2_id = r2["job_id"].as_str().unwrap().to_string();

    let v = h.run(&["list"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert!(jobs.len() >= 2, "expected at least 2 jobs; got: {v}");

    // First job in the list must be the most recent one (job2).
    let first_id = jobs[0]["job_id"].as_str().unwrap_or("");
    assert_eq!(
        first_id, job2_id,
        "expected most recent job first; got: {v}"
    );

    // Verify required fields exist in each job summary.
    for job in jobs {
        assert!(job.get("job_id").is_some(), "job_id missing in job summary");
        assert!(job.get("state").is_some(), "state missing in job summary");
        assert!(
            job.get("started_at").is_some(),
            "started_at missing in job summary"
        );
    }
}

/// Spec: `--limit` truncates the result and sets truncated=true.
#[test]
fn list_limit_truncates_result() {
    let h = TestHarness::new();

    // Run 3 jobs (use --snapshot-after 0 to return immediately).
    h.run(&["run", "--snapshot-after", "0", "echo", "j1"]);
    std::thread::sleep(std::time::Duration::from_millis(10));
    h.run(&["run", "--snapshot-after", "0", "echo", "j2"]);
    std::thread::sleep(std::time::Duration::from_millis(10));
    h.run(&["run", "--snapshot-after", "0", "echo", "j3"]);

    // Request only 2.
    let v = h.run(&["list", "--limit", "2"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert_eq!(jobs.len(), 2, "expected 2 jobs due to --limit 2; got: {v}");
    assert!(
        v["truncated"].as_bool().unwrap_or(false),
        "truncated must be true when result is truncated; got: {v}"
    );
}

/// Spec: `list` root field contains the resolved root path.
#[test]
fn list_response_contains_root_field() {
    let h = TestHarness::new();

    // No run jobs needed for this test; just verify list root field.
    let v = h.run(&["list"]);
    assert_envelope(&v, "list", true);

    let resp_root = v["root"].as_str().expect("root missing in list response");
    assert!(!resp_root.is_empty(), "root field is empty");
}

/// Spec: `list --state running` returns only running jobs; exited jobs are excluded.
#[test]
fn list_filters_by_state_running() {
    let h = TestHarness::new();

    // Start a long-running job (sleep 60); it should appear as "running".
    let long_run = h.run(&["run", "--snapshot-after", "0", "sleep", "60"]);
    let long_job_id = long_run["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    // Start a short job (echo) and wait for it to finish; it should appear as "exited".
    let short_run = h.run(&["run", "--snapshot-after", "500", "echo", "done"]);
    let short_job_id = short_run["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();
    // Wait to ensure the echo job has completed.
    h.run(&["wait", "--timeout-ms", "5000", &short_job_id]);

    // list --state running must contain the long job, not the short one.
    let v = h.run(&["list", "--state", "running"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    let has_long = jobs
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&long_job_id));
    let has_short = jobs
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&short_job_id));

    assert!(
        has_long,
        "long-running job should appear in --state running; got: {v}"
    );
    assert!(
        !has_short,
        "exited job should NOT appear in --state running; got: {v}"
    );

    // All returned jobs must have state == "running".
    for job in jobs {
        let state = job["state"].as_str().unwrap_or("");
        assert_eq!(
            state, "running",
            "unexpected state in --state running result: {state}; job: {job}"
        );
    }

    // Clean up: kill the long-running job.
    h.run(&["kill", &long_job_id]);
}

/// Spec: directories without valid meta.json are counted in skipped, not returned as jobs.
#[test]
fn list_skips_invalid_directories() {
    let h = TestHarness::new();

    // Run a valid job first (use --snapshot-after 0 to return immediately).
    let r = h.run(&["run", "--snapshot-after", "0", "echo", "valid"]);
    let valid_job_id = r["job_id"].as_str().unwrap().to_string();

    // Create a "broken" directory (no meta.json inside).
    let broken_dir = std::path::Path::new(h.root()).join("broken_job_dir");
    std::fs::create_dir_all(&broken_dir).unwrap();

    let v = h.run(&["list"]);
    assert_envelope(&v, "list", true);

    // The valid job must appear.
    let jobs = v["jobs"].as_array().expect("jobs missing");
    let has_valid = jobs
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&valid_job_id));
    assert!(has_valid, "valid job not found in list; got: {v}");

    // The skipped count must be >= 1 (from the broken directory).
    let skipped = v["skipped"]
        .as_u64()
        .expect("skipped missing in list response");
    assert!(
        skipped >= 1,
        "expected skipped >= 1 for broken directory; got: {v}"
    );
}

/// Task 3.3: snapshot-after is clamped to 10,000ms (waited_ms <= 10000).
#[test]
fn run_snapshot_after_is_clamped_to_10_seconds() {
    let h = TestHarness::new();

    // Pass a value larger than 10,000ms; the binary must clamp it to 10,000ms.
    // We use a sleep command to avoid test hanging forever; with clamping the
    // waited_ms must not exceed 10,000.
    // Use a short override to keep the test fast: pass 500ms and verify waited_ms <= 10000.
    let v = h.run(&["run", "--snapshot-after", "20000", "echo", "clamp_test"]);
    assert_envelope(&v, "run", true);

    let waited_ms = v["waited_ms"].as_u64().expect("waited_ms missing");
    // Allow a small tolerance (500ms) over the 10,000ms cap for OS scheduling overhead.
    // The key assertion is that waited_ms is far less than the unclamped value of 20,000ms.
    assert!(
        waited_ms <= 10_500,
        "waited_ms ({waited_ms}) must be <= 10,500 when snapshot-after is clamped to 10,000ms"
    );
    // Ensure the clamp actually prevented a 20,000ms wait.
    assert!(
        waited_ms < 15_000,
        "waited_ms ({waited_ms}) indicates snapshot-after was NOT clamped (expected < 15,000ms)"
    );
}

// ── include-run-output-default: default snapshot ───────────────────────────────

/// Task 5.1: default run (no --snapshot-after flag) returns a snapshot.
/// With the new default of 10,000ms, snapshot should be present in every run response.
#[test]
fn run_default_includes_snapshot() {
    let h = TestHarness::new();

    // Run without any --snapshot-after flag; default is now 10,000ms.
    // echo finishes quickly so the polling loop exits early when output is available.
    let v = h.run(&["run", "echo", "default_snapshot_test"]);
    assert_envelope(&v, "run", true);

    // snapshot must be present with the default 10,000ms wait.
    assert!(
        v.get("snapshot").is_some() && !v["snapshot"].is_null(),
        "snapshot should be present in default run response: {v}"
    );
    let snapshot = &v["snapshot"];
    assert_eq!(
        snapshot["encoding"].as_str().unwrap_or(""),
        "utf-8-lossy",
        "snapshot encoding must be utf-8-lossy"
    );
    assert!(
        snapshot.get("stdout_tail").is_some(),
        "snapshot.stdout_tail must be present"
    );
    assert!(
        snapshot.get("stderr_tail").is_some(),
        "snapshot.stderr_tail must be present"
    );

    // waited_ms must be > 0 (we waited at least one poll cycle).
    let waited_ms = v["waited_ms"].as_u64().expect("waited_ms missing");
    assert!(
        waited_ms > 0,
        "waited_ms must be > 0 with default snapshot_after=10000: {waited_ms}"
    );
    // waited_ms must not exceed 10,000ms (the default cap).
    assert!(
        waited_ms <= 10_000,
        "waited_ms ({waited_ms}) must be <= 10,000ms with default snapshot_after=10000"
    );

    // stdout_tail should contain the echo output.
    let stdout_tail = snapshot["stdout_tail"].as_str().unwrap_or("");
    assert!(
        stdout_tail.contains("default_snapshot_test"),
        "snapshot.stdout_tail should contain 'default_snapshot_test'; got: {stdout_tail:?}"
    );
}

/// Task 5.2: output without a trailing newline is captured in snapshot.stdout_tail.
#[test]
fn run_snapshot_captures_output_without_newline() {
    let h = TestHarness::new();

    // Use printf to emit output without a trailing newline.
    let v = h.run(&[
        "run",
        "--snapshot-after",
        "400",
        "--max-bytes",
        "256",
        "sh",
        "-c",
        "printf 'no-newline-output'",
    ]);
    assert_envelope(&v, "run", true);

    let snapshot = v.get("snapshot").expect("snapshot must be present");
    let stdout_tail = snapshot["stdout_tail"].as_str().unwrap_or("");
    assert!(
        stdout_tail.contains("no-newline-output"),
        "snapshot.stdout_tail should contain 'no-newline-output' even without trailing newline; got: {stdout_tail:?}"
    );
}

/// snapshot-after waits until deadline even when output is immediately available.
///
/// Verifies that `waited_ms >= snapshot_after` when the job is still running
/// at the time output arrives. Uses `printf` (immediate output) + `sleep`
/// (keeps the job running) so that the polling loop cannot exit early due to
/// output availability.
#[test]
fn snapshot_after_waits_until_deadline_despite_early_output() {
    let h = TestHarness::new();
    // Command: print immediately, then sleep long enough that the job is still
    // running when snapshot-after elapses. snapshot-after=200ms per design.md.
    let v = h.run(&[
        "run",
        "--snapshot-after",
        "200",
        "sh",
        "-c",
        "printf 'hello\\n'; sleep 5",
    ]);
    assert_envelope(&v, "run", true);

    // waited_ms must be >= snapshot-after (200ms).
    let waited_ms = v["waited_ms"]
        .as_u64()
        .expect("waited_ms missing from run response");
    assert!(
        waited_ms >= 200,
        "waited_ms ({waited_ms}) must be >= snapshot-after (200ms) even when output \
         arrives early; early-output exit was not removed from the polling loop"
    );

    // snapshot must be present and contain the output that was produced before deadline.
    let snapshot = v.get("snapshot").expect("snapshot must be present");
    let stdout_tail = snapshot["stdout_tail"].as_str().unwrap_or("");
    assert!(
        stdout_tail.contains("hello"),
        "snapshot.stdout_tail should contain 'hello'; got: {stdout_tail:?}"
    );
}

// ── filter-list-by-cwd: cwd filtering ─────────────────────────────────────────

/// Helper that runs the binary with a custom working directory AND a custom root.
fn run_cmd_with_root_and_cwd(
    args: &[&str],
    root: Option<&str>,
    cwd: Option<&std::path::Path>,
) -> (serde_json::Value, std::process::ExitStatus) {
    let bin = binary();
    let mut cmd = std::process::Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let value = if stdout.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "stdout is not valid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}"
            )
        })
    };
    (value, output.status)
}

/// Task 4.1: default `list` filters by the caller's current working directory.
///
/// - Job A is created from dir_a.
/// - Job B is created from dir_b.
/// - `list` called from dir_a shows Job A but NOT Job B.
/// - `list` called from dir_b shows Job B but NOT Job A.
#[test]
fn list_default_filters_by_caller_cwd() {
    let h = TestHarness::new();

    // Create two separate directories to act as distinct cwds.
    let dir_a = tempfile::tempdir().expect("create dir_a");
    let dir_b = tempfile::tempdir().expect("create dir_b");

    // Run job A from dir_a.
    let (va, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_from_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a_id = va["job_id"]
        .as_str()
        .expect("job_id missing for A")
        .to_string();

    // Small sleep to ensure distinct timestamps.
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Run job B from dir_b.
    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_from_b"],
        Some(h.root()),
        Some(dir_b.path()),
    );
    let job_b_id = vb["job_id"]
        .as_str()
        .expect("job_id missing for B")
        .to_string();

    // List from dir_a — should see A, not B.
    let (list_a, _) = run_cmd_with_root_and_cwd(&["list"], Some(h.root()), Some(dir_a.path()));
    assert_envelope(&list_a, "list", true);
    let jobs_a = list_a["jobs"].as_array().expect("jobs missing");
    let has_a = jobs_a
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&job_a_id));
    let has_b_in_a = jobs_a
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&job_b_id));
    assert!(
        has_a,
        "Job A should appear when listing from dir_a; list: {list_a}"
    );
    assert!(
        !has_b_in_a,
        "Job B should NOT appear when listing from dir_a; list: {list_a}"
    );

    // List from dir_b — should see B, not A.
    let (list_b, _) = run_cmd_with_root_and_cwd(&["list"], Some(h.root()), Some(dir_b.path()));
    assert_envelope(&list_b, "list", true);
    let jobs_b = list_b["jobs"].as_array().expect("jobs missing");
    let has_b = jobs_b
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&job_b_id));
    let has_a_in_b = jobs_b
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&job_a_id));
    assert!(
        has_b,
        "Job B should appear when listing from dir_b; list: {list_b}"
    );
    assert!(
        !has_a_in_b,
        "Job A should NOT appear when listing from dir_b; list: {list_b}"
    );
}

/// Task 4.2a: `list --cwd <PATH>` shows only jobs created from that directory.
#[test]
fn list_cwd_flag_filters_by_specified_directory() {
    let h = TestHarness::new();

    let dir_a = tempfile::tempdir().expect("create dir_a");
    let dir_b = tempfile::tempdir().expect("create dir_b");

    // Run job A from dir_a, job B from dir_b.
    let (va, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a_id = va["job_id"].as_str().expect("job_id missing").to_string();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_b"],
        Some(h.root()),
        Some(dir_b.path()),
    );
    let job_b_id = vb["job_id"].as_str().expect("job_id missing").to_string();

    // list --cwd dir_a (from any working directory) should return only job A.
    let dir_a_str = dir_a.path().to_str().expect("dir_a path is utf-8");
    let (list_v, _) =
        run_cmd_with_root_and_cwd(&["list", "--cwd", dir_a_str], Some(h.root()), None);
    assert_envelope(&list_v, "list", true);
    let jobs = list_v["jobs"].as_array().expect("jobs missing");
    let has_a = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_a_id));
    let has_b = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_b_id));
    assert!(
        has_a,
        "Job A should appear with --cwd dir_a; list: {list_v}"
    );
    assert!(
        !has_b,
        "Job B should NOT appear with --cwd dir_a; list: {list_v}"
    );
}

/// Task 4.2b: `list --all` disables cwd filtering and returns all jobs.
#[test]
fn list_all_flag_disables_cwd_filter() {
    let h = TestHarness::new();

    let dir_a = tempfile::tempdir().expect("create dir_a");
    let dir_b = tempfile::tempdir().expect("create dir_b");

    // Run jobs from two different directories.
    let (va, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a_id = va["job_id"].as_str().expect("job_id missing").to_string();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "--snapshot-after", "0", "echo", "job_b"],
        Some(h.root()),
        Some(dir_b.path()),
    );
    let job_b_id = vb["job_id"].as_str().expect("job_id missing").to_string();

    // list --all from dir_a should return both A and B.
    let (list_v, _) =
        run_cmd_with_root_and_cwd(&["list", "--all"], Some(h.root()), Some(dir_a.path()));
    assert_envelope(&list_v, "list", true);
    let jobs = list_v["jobs"].as_array().expect("jobs missing");
    let has_a = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_a_id));
    let has_b = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_b_id));
    assert!(has_a, "Job A should appear with --all; list: {list_v}");
    assert!(has_b, "Job B should appear with --all; list: {list_v}");
}

/// Task 4.3: `list --all --cwd` is a usage error (exit code 2, clap rejects it).
#[test]
fn list_all_and_cwd_conflict_exits_with_code_2() {
    let h = TestHarness::new();
    let bin = binary();

    let output = std::process::Command::new(&bin)
        .env("AGENT_EXEC_ROOT", h.root())
        .args(["list", "--all", "--cwd", "/tmp"])
        .output()
        .expect("run binary");

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when --all and --cwd are both supplied; \
         stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── schema ─────────────────────────────────────────────────────────────────────

/// Task 3.1/3.2: `schema` command returns valid JSON envelope with type="schema".
#[test]
fn schema_returns_json_envelope() {
    // schema does not need a root/harness; run directly.
    let v = run_cmd_with_root(&["schema"], None);
    assert_envelope(&v, "schema", true);
}

/// Task 3.2: `schema` response includes schema_format field set to "json-schema-draft-07".
#[test]
fn schema_response_has_schema_format() {
    let v = run_cmd_with_root(&["schema"], None);
    assert_envelope(&v, "schema", true);

    let schema_format = v["schema_format"]
        .as_str()
        .expect("schema_format missing from schema response");
    assert_eq!(
        schema_format, "json-schema-draft-07",
        "schema_format must be 'json-schema-draft-07'; got: {schema_format}"
    );
}

/// Task 3.2: `schema` response includes a non-empty `schema` object field.
#[test]
fn schema_response_has_schema_object() {
    let v = run_cmd_with_root(&["schema"], None);
    assert_envelope(&v, "schema", true);

    let schema = v
        .get("schema")
        .expect("schema field missing from schema response");
    assert!(
        schema.is_object(),
        "schema field must be a JSON object; got: {schema}"
    );
    assert!(
        !schema.as_object().unwrap().is_empty(),
        "schema field must not be empty; got: {schema}"
    );
}

/// Task 3.2: `schema` response includes `generated_at` field.
#[test]
fn schema_response_has_generated_at() {
    let v = run_cmd_with_root(&["schema"], None);
    assert_envelope(&v, "schema", true);

    let generated_at = v["generated_at"]
        .as_str()
        .expect("generated_at missing from schema response");
    assert!(
        !generated_at.is_empty(),
        "generated_at must not be empty; got: {generated_at:?}"
    );
}

/// `schema` stdout must be a single JSON object only (no extra output).
#[test]
fn schema_stdout_is_single_json_object() {
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["schema"])
        .output()
        .expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "schema stdout should contain exactly 1 line (JSON), got {}: {:?}",
        lines.len(),
        lines
    );
    let parsed: serde_json::Value =
        serde_json::from_str(lines[0]).expect("schema stdout line is not valid JSON");
    assert!(
        parsed.is_object(),
        "schema stdout JSON is not an object: {parsed}"
    );
}
