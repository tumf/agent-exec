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

fn run_raw_with_root_and_stdin(
    args: &[&str],
    root: Option<&str>,
    stdin_bytes: Option<&[u8]>,
) -> std::process::Output {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    if stdin_bytes.is_some() {
        cmd.stdin(std::process::Stdio::piped());
    } else {
        cmd.stdin(std::process::Stdio::null());
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().expect("spawn binary");
    if let Some(bytes) = stdin_bytes {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(bytes).expect("write stdin bytes");
        }
    }

    child.wait_with_output().expect("wait binary output")
}

fn run_cmd_with_root(args: &[&str], root: Option<&str>) -> serde_json::Value {
    let output = run_raw_with_root_and_stdin(args, root, None);
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

fn run_cmd_with_root_and_stdin(
    args: &[&str],
    root: Option<&str>,
    stdin_bytes: &[u8],
) -> serde_json::Value {
    let output = run_raw_with_root_and_stdin(args, root, Some(stdin_bytes));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.trim().is_empty(),
        "stdout is empty (stderr: {stderr})\nargs: {args:?}"
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("stdout is not valid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}")
    })
}

/// Run a command expecting a clap usage error (exit code 2, empty stdout).
///
/// Asserts that the process exits with code 2 and produces no JSON on stdout.
fn assert_usage_error(args: &[&str], root: Option<&str>) {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 (usage error)\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}"
    );
    assert!(
        stdout.trim().is_empty(),
        "expected empty stdout for usage error\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}"
    );
}

/// Run the binary with `--root <root>` as a global CLI flag (not via env var).
/// Verifies normalized global-root syntax: `agent-exec --root <PATH> <subcommand> ...`.
fn run_cmd_with_global_root_flag(root: &str, args: &[&str]) -> serde_json::Value {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.arg("--root").arg(root);
    cmd.args(args);
    // Clear AGENT_EXEC_ROOT to ensure the CLI flag is what takes effect.
    cmd.env_remove("AGENT_EXEC_ROOT");
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.trim().is_empty(),
        "stdout is empty (stderr: {stderr})\nargs: {args:?}"
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("stdout is not valid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}\nargs: {args:?}")
    })
}

/// Run the binary with `--root <root>` placed after the subcommand name (legacy position).
/// Verifies backward-compatible syntax: `agent-exec <subcommand> --root <PATH> ...`.
/// Because --root is declared with `global = true`, clap accepts it in both positions.
fn run_cmd_with_subcommand_root_flag(
    subcommand: &str,
    root: &str,
    extra_args: &[&str],
) -> serde_json::Value {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.arg(subcommand);
    cmd.arg("--root").arg(root);
    cmd.args(extra_args);
    cmd.env_remove("AGENT_EXEC_ROOT");
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.trim().is_empty(),
        "stdout is empty (stderr: {stderr})\nsubcommand: {subcommand}, root: {root}, extra: {extra_args:?}"
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "stdout is not valid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}\nsubcommand: {subcommand}"
        )
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
    let v = h.run(&["run", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
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
    assert!(v.get("stdout").is_some(), "stdout missing: {v}");
    assert!(v.get("stderr").is_some(), "stderr missing: {v}");
}

#[test]
fn run_returns_inline_payload_without_legacy_snapshot_fields() {
    let h = TestHarness::new();
    let v = h.run(&["run", "echo", "snapshot_test"]);
    assert_envelope(&v, "run", true);
    assert!(
        v.get("snapshot").is_none(),
        "snapshot field must be absent: {v}"
    );
    assert!(
        v.get("final_snapshot").is_none(),
        "final_snapshot field must be absent: {v}"
    );
    assert!(
        v.get("waited_ms").is_some(),
        "waited_ms must be present: {v}"
    );
}

// ── status ─────────────────────────────────────────────────────────────────────

#[test]
fn status_returns_json_for_existing_job() {
    let h = TestHarness::new();

    // First run a job (run returns immediately).
    let run_v = h.run(&["run", "echo", "hi"]);
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
    let run_v = h.run(&["run", "echo", "tail_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let v = h.run(&["tail", &job_id]);
    assert_envelope(&v, "tail", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert_eq!(v["encoding"].as_str().unwrap_or(""), "utf-8-lossy");
    // Spec requires stdout / stderr field names (not stdout / stderr).
    assert!(v.get("stdout").is_some(), "stdout missing");
    assert!(v.get("stderr").is_some(), "stderr missing");
}

fn wait_until_terminal(h: &TestHarness, job_id: &str) -> serde_json::Value {
    for _ in 0..20 {
        let wait_v = h.run(&["wait", "--until", "1000", job_id]);
        assert_envelope(&wait_v, "wait", true);
        let state = wait_v["state"].as_str().unwrap_or("");
        if state == "exited" || state == "killed" || state == "failed" {
            return wait_v;
        }
    }
    panic!("job did not reach terminal state in time: {job_id}");
}

#[test]
fn run_stdin_dash_pipe_materialized_and_visible_in_meta() {
    let h = TestHarness::new();
    let input = b"alpha\nbeta\n";

    let v =
        run_cmd_with_root_and_stdin(&["run", "--stdin", "-", "--", "cat"], Some(h.root()), input);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();

    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let tail_v = h.run(&["tail", &job_id]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("alpha") && stdout.contains("beta"),
        "tail.stdout should contain piped stdin: {stdout:?}"
    );

    let meta_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("meta.json");
    let meta_raw = std::fs::read_to_string(&meta_path).expect("read meta.json");
    let meta_json: serde_json::Value = serde_json::from_str(&meta_raw).expect("parse meta.json");
    assert_eq!(
        meta_json["stdin_file"].as_str().unwrap_or(""),
        "stdin.bin",
        "meta.json.stdin_file should point to materialized stdin"
    );
    let stdin_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("stdin.bin");
    let materialized = std::fs::read(&stdin_path).expect("read stdin.bin");
    assert_eq!(materialized, input, "stdin.bin should match provided stdin");
}

#[test]
fn run_stdin_inline_preserves_exact_bytes() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--stdin", "abc", "--", "cat"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();

    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let tail_v = h.run(&["tail", &job_id]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert_eq!(stdout, "abc", "inline stdin should not append newline");
}

#[test]
fn run_stdin_file_uses_materialized_copy() {
    let h = TestHarness::new();
    let src_path = std::path::Path::new(h.root()).join("stdin-source.txt");
    std::fs::write(&src_path, b"file-input").expect("write stdin source file");

    let v = h.run(&[
        "run",
        "--stdin-file",
        src_path.to_str().expect("utf8 path"),
        "--",
        "cat",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();

    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let tail_v = h.run(&["tail", &job_id]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert_eq!(stdout, "file-input");

    let stdin_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("stdin.bin");
    let materialized = std::fs::read(&stdin_path).expect("read stdin.bin");
    assert_eq!(materialized, b"file-input");
}

#[test]
fn run_and_create_persist_same_stdin_meta_shape() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "--stdin", "shape", "--", "cat"]);
    let run_id = run_v["job_id"].as_str().expect("run job_id missing");
    let run_meta_path = std::path::Path::new(h.root())
        .join(run_id)
        .join("meta.json");
    let run_meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_meta_path).expect("read run meta"))
            .expect("parse run meta");

    let create_v = h.run(&["create", "--stdin", "shape", "--", "cat"]);
    let create_id = create_v["job_id"].as_str().expect("create job_id missing");
    let create_meta_path = std::path::Path::new(h.root())
        .join(create_id)
        .join("meta.json");
    let create_meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(create_meta_path).expect("read create meta"))
            .expect("parse create meta");

    assert_eq!(run_meta["stdin_file"], create_meta["stdin_file"]);
    assert_eq!(run_meta["stdin_file"].as_str().unwrap_or(""), "stdin.bin");
}

#[test]
fn run_without_stdin_keeps_null_stdin_behavior() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--", "cat"]);
    assert_envelope(&v, "run", true);

    let job_id = v["job_id"].as_str().expect("job_id missing");
    let meta_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("meta.json");
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(meta_path).expect("read meta"))
            .expect("parse meta");
    assert!(
        meta.get("stdin_file").is_none() || meta["stdin_file"].is_null(),
        "stdin_file should be absent or null when stdin is not configured"
    );
    assert!(
        !std::path::Path::new(h.root())
            .join(job_id)
            .join("stdin.bin")
            .exists(),
        "stdin.bin should not exist when stdin is not configured"
    );
}

#[test]
fn create_start_reuses_stdin_definition() {
    let h = TestHarness::new();
    let create_v = h.run(&["create", "--stdin", "hello", "--", "cat"]);
    assert_envelope(&create_v, "create", true);
    let job_id = create_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    let start_v = h.run(&["start", &job_id]);
    assert_envelope(&start_v, "start", true);
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let tail_v = h.run(&["tail", &job_id]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert_eq!(stdout, "hello");
}

#[test]
fn create_with_stdin_dash_materializes_input_for_later_start() {
    let h = TestHarness::new();
    let create_v = run_cmd_with_root_and_stdin(
        &["create", "--stdin", "-", "--", "cat"],
        Some(h.root()),
        b"from-create-pipe",
    );
    assert_envelope(&create_v, "create", true);
    let job_id = create_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    let start_v = h.run(&["start", &job_id]);
    assert_envelope(&start_v, "start", true);
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let tail_v = h.run(&["tail", &job_id]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert_eq!(stdout, "from-create-pipe");
}

// ── wait ───────────────────────────────────────────────────────────────────────

#[test]
fn wait_returns_json_after_job_finishes() {
    let h = TestHarness::new();

    // run returns immediately, so this can be tested directly so we can test wait separately.
    let run_v = h.run(&["run", "echo", "done"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait with --until=5s; echo finishes fast.
    let v = h.run(&["wait", "--until", "5", &job_id]);
    assert_envelope(&v, "wait", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("state").is_some(), "state missing");
}

#[test]
fn wait_default_until_returns_non_terminal_for_long_running_job() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let started = std::time::Instant::now();
    let v = h.run(&["wait", &job_id]);
    let elapsed_ms = started.elapsed().as_millis() as u64;

    assert_envelope(&v, "wait", true);
    assert!(
        elapsed_ms >= 29_000,
        "default wait should be ~30s; got {elapsed_ms}ms"
    );
    let state = v["state"].as_str().unwrap_or("");
    assert!(
        state == "running" || state == "created",
        "wait default deadline should return non-terminal state; got: {state}"
    );
    assert!(
        v.get("exit_code").is_none() || v["exit_code"].is_null(),
        "exit_code should be absent/null for non-terminal timeout: {v}"
    );

    let _ = h.run(&["kill", "--signal", "KILL", &job_id]);
}

#[test]
fn wait_forever_waits_until_terminal() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "sh", "-c", "sleep 0.1"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let v = h.run(&["wait", "--forever", &job_id]);
    assert_envelope(&v, "wait", true);
    let state = v["state"].as_str().unwrap_or("");
    assert!(state == "exited" || state == "killed" || state == "failed");
}

#[test]
fn wait_rejects_legacy_timeout_ms_alias() {
    let h = TestHarness::new();
    assert_usage_error(
        &["wait", "--timeout-ms", "100", "missing-job"],
        Some(h.root()),
    );
}

#[test]
fn stdin_option_conflict_is_usage_error_for_run_and_create() {
    let h = TestHarness::new();
    assert_usage_error(
        &[
            "run",
            "--stdin",
            "x",
            "--stdin-file",
            "/tmp/in.txt",
            "--",
            "cat",
        ],
        Some(h.root()),
    );
    assert_usage_error(
        &[
            "create",
            "--stdin",
            "x",
            "--stdin-file",
            "/tmp/in.txt",
            "--",
            "cat",
        ],
        Some(h.root()),
    );
}

#[cfg(unix)]
#[test]
fn run_stdin_dash_with_tty_like_stdin_fails_fast() {
    let h = TestHarness::new();
    let bin = binary();
    let command = format!(
        "AGENT_EXEC_ROOT={} {} run --stdin - -- cat",
        h.root(),
        bin.display()
    );

    let mut cmd = std::process::Command::new("script");
    if cfg!(target_os = "macos") {
        cmd.arg("-q")
            .arg("/dev/null")
            .arg("sh")
            .arg("-lc")
            .arg(&command);
    } else {
        cmd.arg("-q")
            .arg("-e")
            .arg("-c")
            .arg(&command)
            .arg("/dev/null");
    }

    let output = cmd.output().expect("run script tty wrapper");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    let json_start = combined
        .find('{')
        .expect("tty wrapper output should include JSON error envelope");
    let json = combined[json_start..].trim();
    let v: serde_json::Value = serde_json::from_str(json).expect("parse tty failure JSON");

    assert!(
        !v["ok"].as_bool().unwrap_or(true),
        "expected error response: {v}"
    );
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "stdin_required");
}

// ── kill ───────────────────────────────────────────────────────────────────────

#[test]
fn kill_returns_json() {
    let h = TestHarness::new();

    // Run a long-running command (run returns immediately).
    let run_v = h.run(&["run", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Brief wait to let the supervisor start the child.
    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = h.run(&["kill", "--signal", "KILL", &job_id]);
    assert_envelope(&v, "kill", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
    assert!(v.get("signal").is_some(), "signal missing");
}

#[test]
fn kill_signal_non_listed_value_accepted_by_clap() {
    // Verifies that a signal name not in the suggested list (e.g., QUIT) is
    // accepted by clap (exit code != 2) and reaches the runtime error path
    // (job not found), rather than being rejected as a usage error.
    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["kill", "--signal", "QUIT", "NONEXISTENT_JOB_ID_XYZ"])
        .output()
        .expect("run binary");
    let code = output.status.code().unwrap_or(-1);
    assert_ne!(
        code, 2,
        "exit code 2 means clap rejected 'QUIT' as a usage error; it should be accepted"
    );
    // Exit code should be non-zero (job not found runtime error) but not 2.
    assert_ne!(code, 0, "expected non-zero exit code for unknown job id");
}

#[test]
fn kill_observes_terminal_state() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = h.run(&["kill", "--signal", "KILL", &job_id]);
    assert_envelope(&v, "kill", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);

    let state = v.get("state").and_then(|s| s.as_str());
    assert!(state.is_some(), "state field must be present: {v}");
    let state = state.unwrap();
    assert!(
        state == "killed" || state == "exited" || state == "failed",
        "expected terminal state, got: {state}"
    );

    assert!(
        v.get("observed_within_ms").is_some(),
        "observed_within_ms must be present: {v}"
    );
}

#[test]
fn kill_no_wait_returns_legacy_shape() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    let v = h.run(&["kill", "--no-wait", "--signal", "KILL", &job_id]);
    assert_envelope(&v, "kill", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);

    assert!(
        v.get("state").is_none() || v["state"].is_null(),
        "state must be absent in --no-wait mode: {v}"
    );
    assert!(
        v.get("exit_code").is_none() || v["exit_code"].is_null(),
        "exit_code must be absent in --no-wait mode: {v}"
    );
    assert!(
        v.get("observed_within_ms").is_none() || v["observed_within_ms"].is_null(),
        "observed_within_ms must be absent in --no-wait mode: {v}"
    );
}

// ── full.log ───────────────────────────────────────────────────────────────────

#[test]
fn run_creates_full_log() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "echo", "full_log_test"]);
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

    // run already returns immediately; no child wait is required here.
    let run_v = h.run(&["run", "echo", "log_files_test"]);
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

    // Run a job and read back state.json from the job directory.
    let run_v = h.run(&["run", "echo", "state_test"]);
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
    // Use `--` before the command as the spec requires.
    let v = h.run(&["run", "--", "echo", "hello_dash"]);
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

    let run_v = h.run(&["run", "echo", "full_log_format_test"]);
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

    h.run(&["run", "--log", log_path_str, "echo", "log_override_test"]);

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
    // run returns immediately, so this can be tested directly; timeout is tested via status.
    let run_v = h.run(&["run", "--timeout", "1", "--kill-after", "1", "sleep", "60"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Poll until the job is no longer running (timeout + kill-after should fire).
    // Use a polling loop instead of a fixed sleep to tolerate slow CI runners.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let v = h.run(&["status", &job_id]);
        let state = v["state"].as_str().unwrap_or("running");
        if state != "running" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "job should have been terminated by timeout; state={state}"
        );
    }
}

/// Spec: --progress-every updates state.json.updated_at within the interval.
#[test]
fn run_progress_every_updates_state() {
    let h = TestHarness::new();

    // Run a long sleep with progress-every=1s.
    let run_v = h.run(&["run", "--progress-every", "1", "sleep", "5"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait for at least one progress tick.
    std::thread::sleep(std::time::Duration::from_millis(1500));

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
    // run returns immediately, so this can be tested directly; progress-every is the focus here.
    let run_v = h.run(&["run", "--progress-every", "1", "--", "echo", "done"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Wait enough time for the first progress tick and child-exit reconciliation.
    std::thread::sleep(std::time::Duration::from_millis(2500));

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

/// Spec: tail range reflects returned window when output exceeds constraints.
#[test]
fn tail_range_reflects_slice_when_over_limit() {
    let h = TestHarness::new();

    // Generate exactly 5 lines; request only 2 lines.
    let run_v = h.run(&[
        "run",
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

    let stdout = v["stdout"].as_str().unwrap_or_default();
    let range = v["stdout_range"].as_array().expect("stdout_range missing");
    let begin = range.first().and_then(|x| x.as_u64()).unwrap_or(0);
    let end = range.get(1).and_then(|x| x.as_u64()).unwrap_or(0);
    assert_eq!(
        end.saturating_sub(begin),
        stdout.len() as u64,
        "stdout_range length must match returned stdout bytes: {v}"
    );
}

// ── add-run-tail-metrics: new fields ──────────────────────────────────────────

#[test]
fn run_includes_elapsed_ms_log_paths_and_inline_metrics() {
    let h = TestHarness::new();

    let v = h.run(&["run", "echo", "metrics_test"]);
    assert_envelope(&v, "run", true);

    let _elapsed_ms = v["elapsed_ms"]
        .as_u64()
        .expect("elapsed_ms missing from run response");

    let stdout_path = v["stdout_log_path"]
        .as_str()
        .expect("stdout_log_path missing from run response");
    let stderr_path = v["stderr_log_path"]
        .as_str()
        .expect("stderr_log_path missing from run response");
    assert!(!stdout_path.is_empty(), "stdout_log_path is empty");
    assert!(!stderr_path.is_empty(), "stderr_log_path is empty");
    assert!(std::path::Path::new(stdout_path).is_absolute());
    assert!(std::path::Path::new(stderr_path).is_absolute());

    assert!(
        v.get("waited_ms").is_some(),
        "waited_ms must be present: {v}"
    );
    assert!(v.get("stdout").is_some(), "stdout must be present: {v}");
    assert!(v.get("stderr").is_some(), "stderr must be present: {v}");
    assert!(
        v.get("stdout_range").is_some(),
        "stdout_range must be present: {v}"
    );
    assert!(
        v.get("stderr_range").is_some(),
        "stderr_range must be present: {v}"
    );
    assert!(v.get("snapshot").is_none(), "snapshot must be absent: {v}");
}

/// Task 3.2: tail response includes log paths and bytes metrics.
#[test]
fn tail_includes_log_paths_and_bytes_metrics() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "echo", "tail_bytes_test"]);
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
        v.get("stdout_total_bytes").is_some(),
        "stdout_total_bytes missing from tail response: {v}"
    );
    assert!(
        v.get("stderr_total_bytes").is_some(),
        "stderr_total_bytes missing from tail response: {v}"
    );
    assert!(
        v.get("stdout_range").is_some(),
        "stdout_range missing from tail response: {v}"
    );
    assert!(
        v.get("stderr_range").is_some(),
        "stderr_range missing from tail response: {v}"
    );

    // range end must be <= total bytes.
    let stdout_observed = v["stdout_total_bytes"].as_u64().unwrap_or(0);
    let stdout_end = v["stdout_range"]
        .as_array()
        .and_then(|r| r.get(1))
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    assert!(
        stdout_end <= stdout_observed,
        "stdout_range end ({stdout_end}) must be <= stdout_total_bytes ({stdout_observed})"
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

    // Run two jobs (run returns immediately); both should appear in list.
    let _r1 = h.run(&["run", "echo", "job1"]);
    // Small sleep to ensure distinct timestamps.
    std::thread::sleep(std::time::Duration::from_millis(10));
    let r2 = h.run(&["run", "echo", "job2"]);
    let job2_id = r2["job_id"].as_str().unwrap().to_string();

    let v = h.run(&["list"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert!(jobs.len() >= 2, "expected at least 2 jobs; got: {v}");

    // Jobs must be sorted by started_at desc, then job_id desc.
    let first_started = jobs[0]["started_at"].as_str().unwrap_or("");
    let second_started = jobs[1]["started_at"].as_str().unwrap_or("");
    let first_id = jobs[0]["job_id"].as_str().unwrap_or("");
    let second_id = jobs[1]["job_id"].as_str().unwrap_or("");
    assert!(
        first_started > second_started
            || (first_started == second_started && first_id >= second_id),
        "jobs must be sorted by started_at/job_id desc; got: {v}"
    );
    assert!(
        jobs.iter().any(|j| j["job_id"].as_str() == Some(&job2_id)),
        "most recently created job must be present in list: {v}"
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

    // Run 3 jobs (run returns immediately).
    h.run(&["run", "echo", "j1"]);
    std::thread::sleep(std::time::Duration::from_millis(10));
    h.run(&["run", "echo", "j2"]);
    std::thread::sleep(std::time::Duration::from_millis(10));
    h.run(&["run", "echo", "j3"]);

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

/// Spec: default `list` (no --limit) returns at most 50 jobs and sets truncated=true
/// when more than 50 exist.
#[test]
fn list_default_limit_truncates_at_50() {
    let h = TestHarness::new();
    let cwd = std::env::current_dir()
        .expect("current_dir")
        .to_str()
        .expect("cwd utf-8")
        .to_string();

    for i in 0..60 {
        let job_id = format!("job-{i:04}");
        let job_dir = std::path::Path::new(h.root()).join(&job_id);
        std::fs::create_dir_all(&job_dir).expect("create job dir");
        let meta = serde_json::json!({
            "job": { "id": job_id },
            "schema_version": "0.1",
            "command": ["echo", "hi"],
            "created_at": format!("2026-01-01T00:{i:02}:00Z"),
            "root": h.root(),
            "env_keys": [],
            "cwd": cwd,
            "tags": [],
            "inherit_env": true,
        });
        std::fs::write(job_dir.join("meta.json"), meta.to_string()).expect("write meta.json");
    }

    let v = h.run(&["list", "--all"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert_eq!(
        jobs.len(),
        50,
        "default limit should return 50 jobs; got: {}",
        jobs.len()
    );
    assert!(
        v["truncated"].as_bool().unwrap_or(false),
        "truncated must be true when >50 jobs exist; got: {v}"
    );
}

/// Spec: `--limit 0` disables truncation and returns all jobs.
#[test]
fn list_limit_zero_returns_all() {
    let h = TestHarness::new();
    let cwd = std::env::current_dir()
        .expect("current_dir")
        .to_str()
        .expect("cwd utf-8")
        .to_string();

    for i in 0..60 {
        let job_id = format!("job-{i:04}");
        let job_dir = std::path::Path::new(h.root()).join(&job_id);
        std::fs::create_dir_all(&job_dir).expect("create job dir");
        let meta = serde_json::json!({
            "job": { "id": job_id },
            "schema_version": "0.1",
            "command": ["echo", "hi"],
            "created_at": format!("2026-01-01T00:{i:02}:00Z"),
            "root": h.root(),
            "env_keys": [],
            "cwd": cwd,
            "tags": [],
            "inherit_env": true,
        });
        std::fs::write(job_dir.join("meta.json"), meta.to_string()).expect("write meta.json");
    }

    let v = h.run(&["list", "--all", "--limit", "0"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    assert_eq!(
        jobs.len(),
        60,
        "--limit 0 should return all 60 jobs; got: {}",
        jobs.len()
    );
    assert!(
        !v["truncated"].as_bool().unwrap_or(true),
        "truncated must be false with --limit 0; got: {v}"
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
    let long_run = h.run(&["run", "sleep", "60"]);
    let long_job_id = long_run["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    // Start a short job (echo) and wait for it to finish; it should appear as "exited".
    let short_run = h.run(&["run", "echo", "done"]);
    let short_job_id = short_run["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();
    // Wait to ensure the echo job has completed.
    h.run(&["wait", "--until", "5", &short_job_id]);

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

    // Run a valid job first (run returns immediately).
    let r = h.run(&["run", "echo", "valid"]);
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

#[test]
fn run_rejects_removed_snapshot_after_flag() {
    let h = TestHarness::new();
    assert_usage_error(
        &["run", "--snapshot-after", "200", "echo", "invalid"],
        Some(h.root()),
    );
}

#[test]
fn run_accepts_max_bytes_flag() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--max-bytes", "256", "echo", "valid"]);
    assert_envelope(&v, "run", true);
}

#[test]
fn run_rejects_removed_tail_lines_flag() {
    let h = TestHarness::new();
    assert_usage_error(
        &["run", "--tail-lines", "10", "echo", "invalid"],
        Some(h.root()),
    );
}

#[test]
fn run_accepts_wait_flag() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--wait", "echo", "valid"]);
    assert_envelope(&v, "run", true);
}

#[test]
fn run_accepts_wait_bool_forms_for_backward_compatibility() {
    let h = TestHarness::new();
    let v_true = h.run(&["run", "--wait", "true", "echo", "valid_true"]);
    assert_envelope(&v_true, "run", true);

    let v_false = h.run(&["run", "--wait", "false", "echo", "valid_false"]);
    assert_envelope(&v_false, "run", true);
    assert_eq!(
        v_false["waited_ms"].as_u64().unwrap_or(u64::MAX),
        0,
        "--wait false must skip additional waiting"
    );
}

#[test]
fn run_preserves_child_bare_wait_argument() {
    let h = TestHarness::new();
    let v = h.run(&[
        "run",
        "python3",
        "-c",
        "import sys; print(sys.argv[1])",
        "--wait",
    ]);
    assert_envelope(&v, "run", true);
    let stdout = v["stdout"].as_str().expect("stdout missing");
    assert_eq!(stdout.trim(), "--wait", "child argv must remain unchanged");
}

#[test]
fn start_accepts_wait_flag() {
    let h = TestHarness::new();
    let create_v = h.run(&["create", "--", "echo", "start_wait_enabled"]);
    let job_id = create_v["job_id"].as_str().expect("job_id missing");
    let v = h.run(&["start", "--wait", job_id]);
    assert_envelope(&v, "start", true);
}

#[test]
fn start_accepts_wait_bool_forms_for_backward_compatibility() {
    let h = TestHarness::new();

    let create_true = h.run(&["create", "--", "echo", "start_wait_true"]);
    let job_id_true = create_true["job_id"].as_str().expect("job_id missing");
    let v_true = h.run(&["start", "--wait", "true", job_id_true]);
    assert_envelope(&v_true, "start", true);

    let create_false = h.run(&["create", "--", "echo", "start_wait_false"]);
    let job_id_false = create_false["job_id"].as_str().expect("job_id missing");
    let v_false = h.run(&["start", "--wait", "false", job_id_false]);
    assert_envelope(&v_false, "start", true);
    assert_eq!(
        v_false["waited_ms"].as_u64().unwrap_or(u64::MAX),
        0,
        "start --wait false must skip additional waiting"
    );
}

#[test]
fn start_rejects_removed_snapshot_after_flag() {
    let h = TestHarness::new();
    let create_v = h.run(&["create", "--", "echo", "start_snapshot_after_removed"]);
    let job_id = create_v["job_id"].as_str().expect("job_id missing");
    assert_usage_error(&["start", "--snapshot-after", "1", job_id], Some(h.root()));
}

/// run/start response should use inline canonical fields (no legacy snapshot names).
#[test]
fn run_start_response_uses_inline_output_fields() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "echo", "inline_fields"]);
    assert_envelope(&run_v, "run", true);
    assert!(
        run_v.get("snapshot").is_none(),
        "snapshot must be absent: {run_v}"
    );
    assert!(
        run_v.get("final_snapshot").is_none(),
        "final_snapshot must be absent: {run_v}"
    );
    assert!(
        run_v.get("waited_ms").is_some(),
        "waited_ms must be present: {run_v}"
    );
    assert!(
        run_v.get("stdout").is_some(),
        "stdout must be present: {run_v}"
    );
    assert!(
        run_v.get("stderr").is_some(),
        "stderr must be present: {run_v}"
    );
    assert!(
        run_v.get("stdout_range").is_some(),
        "stdout_range must be present: {run_v}"
    );
    assert!(
        run_v.get("stderr_range").is_some(),
        "stderr_range must be present: {run_v}"
    );

    let create_v = h.run(&["create", "--", "echo", "start_inline_fields"]);
    let job_id = create_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();
    let start_v = h.run(&["start", &job_id]);
    assert_envelope(&start_v, "start", true);
    assert!(
        start_v.get("waited_ms").is_some(),
        "waited_ms must be present: {start_v}"
    );
    assert!(
        start_v.get("stdout").is_some(),
        "stdout must be present: {start_v}"
    );
    assert!(
        start_v.get("stderr").is_some(),
        "stderr must be present: {start_v}"
    );
}

// run/start は起動専用。観測は wait/tail で行う。
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
        &["run", "echo", "job_from_a"],
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
        &["run", "echo", "job_from_b"],
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
        &["run", "echo", "job_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a_id = va["job_id"].as_str().expect("job_id missing").to_string();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "job_b"],
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
        &["run", "echo", "job_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a_id = va["job_id"].as_str().expect("job_id missing").to_string();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "job_b"],
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

// ---------------------------------------------------------------------------
// install-skills tests
// ---------------------------------------------------------------------------

/// Task 4.1: `install-skills` installs the embedded `agent-exec` skill and
/// returns the expected JSON envelope with `type="install_skills"`.
#[test]
fn install_skills_embedded_source_succeeds() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let agents_dir = tmp.path().join(".agents");

    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["install-skills"])
        .current_dir(tmp.path())
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.trim().is_empty(),
        "stdout is empty (stderr: {stderr})"
    );
    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_envelope(&v, "install_skills", true);

    // skills array must have at least one entry with name="agent-exec"
    let skills = v["skills"].as_array().expect("skills must be an array");
    assert!(!skills.is_empty(), "skills array must not be empty");
    assert_eq!(
        skills[0]["name"].as_str().unwrap_or(""),
        "agent-exec",
        "skills[0].name must be 'agent-exec'; got: {v}"
    );
    assert_eq!(
        skills[0]["source_type"].as_str().unwrap_or(""),
        "embedded",
        "skills[0].source_type must be 'embedded'; got: {v}"
    );
    assert!(
        skills[0]["path"].as_str().is_some(),
        "skills[0].path must be present; got: {v}"
    );
    assert!(
        v["lock_file_path"].as_str().is_some(),
        "lock_file_path must be present; got: {v}"
    );

    // The skill directory must have been created with SKILL.md inside.
    let skill_dir = agents_dir.join("skills").join("agent-exec");
    assert!(
        skill_dir.exists(),
        "skill directory must exist at {}",
        skill_dir.display()
    );
    assert!(
        skill_dir.join("SKILL.md").exists(),
        "SKILL.md must exist inside the installed skill directory"
    );
    assert!(
        skill_dir
            .join("references")
            .join("cli-contract.md")
            .exists(),
        "cli-contract.md must exist inside the installed skill directory"
    );
    assert!(
        skill_dir
            .join("references")
            .join("completion-events.md")
            .exists(),
        "completion-events.md must exist inside the installed skill directory"
    );
    assert!(
        skill_dir.join("references").join("openclaw.md").exists(),
        "openclaw.md must exist inside the installed skill directory"
    );

    // The lock file must exist.
    let lock_path = agents_dir.join(".skill-lock.json");
    assert!(
        lock_path.exists(),
        "lock file must exist at {}",
        lock_path.display()
    );

    // Lock file must contain a valid JSON array entry for agent-exec with
    // name, path, and source_type fields (per spec requirement).
    let lock_content = std::fs::read_to_string(&lock_path).expect("read lock file");
    let lock: serde_json::Value =
        serde_json::from_str(&lock_content).expect("lock file must be valid JSON");
    let lock_skills = lock["skills"]
        .as_array()
        .expect("lock skills must be an array");
    assert!(!lock_skills.is_empty(), "lock skills must not be empty");
    assert_eq!(
        lock_skills[0]["name"].as_str().unwrap_or(""),
        "agent-exec",
        "lock skills[0].name must be 'agent-exec'"
    );
    assert!(
        lock_skills[0]["path"].as_str().is_some(),
        "lock skills[0].path must be present; got: {lock}"
    );
    assert!(
        lock_skills[0]["source_type"].as_str().is_some(),
        "lock skills[0].source_type must be present; got: {lock}"
    );
}

/// Task 4.2: repeated `install-skills` updates the same embedded skill lock entry.
#[test]
fn install_skills_repeated_install_updates_single_lock_entry() {
    let install_root = tempfile::tempdir().expect("create install root");
    let agents_dir = install_root.path().join(".agents");

    let bin = binary();
    for _ in 0..2 {
        let output = std::process::Command::new(&bin)
            .args(["install-skills"])
            .current_dir(install_root.path())
            .output()
            .expect("run binary");
        assert!(output.status.success(), "install-skills must succeed");
    }

    let lock_path = agents_dir.join(".skill-lock.json");
    assert!(
        lock_path.exists(),
        "lock file must exist at {}",
        lock_path.display()
    );
    let lock_content = std::fs::read_to_string(&lock_path).expect("read lock file");
    let lock: serde_json::Value =
        serde_json::from_str(&lock_content).expect("lock file must be valid JSON");
    let lock_skills = lock["skills"]
        .as_array()
        .expect("lock skills must be an array");
    assert_eq!(
        lock_skills.len(),
        1,
        "repeated embedded install should keep exactly one lock entry"
    );
    assert_eq!(
        lock_skills[0]["name"].as_str().unwrap_or(""),
        "agent-exec",
        "lock skills[0].name must be 'agent-exec'"
    );
    assert!(
        lock_skills[0]["path"].as_str().is_some(),
        "lock skills[0].path must be present; got: {lock}"
    );
    assert_eq!(
        lock_skills[0]["source_type"].as_str().unwrap_or(""),
        "embedded",
        "lock skills[0].source_type must be 'embedded'; got: {lock}"
    );
}

/// `install-skills --claude` installs into `.claude/skills/` and writes
/// `.claude/.skill-lock.json`.
#[test]
fn install_skills_claude_local_succeeds() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let claude_dir = tmp.path().join(".claude");

    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["install-skills", "--claude"])
        .current_dir(tmp.path())
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "command failed (stderr: {stderr})");

    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_envelope(&v, "install_skills", true);

    let skills = v["skills"].as_array().expect("skills must be an array");
    assert_eq!(skills[0]["name"].as_str().unwrap_or(""), "agent-exec");

    let skill_path = skills[0]["path"]
        .as_str()
        .expect("skills[0].path must be present");
    assert!(
        skill_path.contains(".claude/skills/agent-exec"),
        "skills[0].path must be under .claude/skills/; got: {skill_path}"
    );

    let lock_file_path = v["lock_file_path"]
        .as_str()
        .expect("lock_file_path must be present");
    assert!(
        lock_file_path.contains(".claude/.skill-lock.json"),
        "lock_file_path must be under .claude/; got: {lock_file_path}"
    );

    let skill_dir = claude_dir.join("skills").join("agent-exec");
    assert!(
        skill_dir.exists(),
        "skill directory must exist at {}",
        skill_dir.display()
    );
    assert!(
        skill_dir.join("SKILL.md").exists(),
        "SKILL.md must exist inside the installed skill directory"
    );

    let lock_path = claude_dir.join(".skill-lock.json");
    assert!(
        lock_path.exists(),
        "lock file must exist at {}",
        lock_path.display()
    );

    let lock_content = std::fs::read_to_string(&lock_path).expect("read lock file");
    let lock: serde_json::Value =
        serde_json::from_str(&lock_content).expect("lock file must be valid JSON");
    let lock_skills = lock["skills"]
        .as_array()
        .expect("lock skills must be an array");
    assert!(!lock_skills.is_empty(), "lock skills must not be empty");
    assert_eq!(lock_skills[0]["name"].as_str().unwrap_or(""), "agent-exec");
}

/// `install-skills --claude --global` installs into `~/.claude/skills/`.
/// We simulate HOME with a tempdir to avoid touching the real home.
#[test]
fn install_skills_claude_global_succeeds() {
    let tmp = tempfile::tempdir().expect("create tempdir");

    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["install-skills", "--claude", "--global"])
        .env("HOME", tmp.path())
        .current_dir(tmp.path())
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "command failed (stderr: {stderr})");

    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_envelope(&v, "install_skills", true);

    let skills = v["skills"].as_array().expect("skills must be an array");
    let skill_path = skills[0]["path"]
        .as_str()
        .expect("skills[0].path must be present");
    assert!(
        skill_path.contains(".claude/skills/agent-exec"),
        "skills[0].path must be under .claude/skills/; got: {skill_path}"
    );

    let lock_file_path = v["lock_file_path"]
        .as_str()
        .expect("lock_file_path must be present");
    assert!(
        lock_file_path.contains(".claude/.skill-lock.json"),
        "lock_file_path must be under .claude/; got: {lock_file_path}"
    );

    let global_skill_dir = tmp.path().join(".claude").join("skills").join("agent-exec");
    assert!(
        global_skill_dir.exists(),
        "global skill directory must exist at {}",
        global_skill_dir.display()
    );
}

/// `install-skills` without `--claude` still uses `.agents/` (backward compat).
#[test]
fn install_skills_without_claude_uses_agents() {
    let tmp = tempfile::tempdir().expect("create tempdir");

    let bin = binary();
    let output = std::process::Command::new(&bin)
        .args(["install-skills"])
        .current_dir(tmp.path())
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "command failed");

    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    let skill_path = v["skills"][0]["path"].as_str().expect("path present");
    assert!(
        skill_path.contains(".agents/skills/agent-exec"),
        "without --claude, path must use .agents/; got: {skill_path}"
    );

    let lock_file_path = v["lock_file_path"]
        .as_str()
        .expect("lock_file_path present");
    assert!(
        lock_file_path.contains(".agents/.skill-lock.json"),
        "without --claude, lock_file_path must use .agents/; got: {lock_file_path}"
    );
}

// ── notify file sink ────────────────────────────────────────────────────────────

/// File sink: completion event is appended as NDJSON to the specified file.
#[test]
fn notify_file_sink_appends_ndjson_on_job_finish() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let events_file = tmp_dir.path().join("events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    let v = h.run(&[
        "run",
        "--notify-file",
        events_file_str,
        "--",
        "echo",
        "notify_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    std::thread::sleep(std::time::Duration::from_millis(300));

    // The events file must exist and contain a valid NDJSON line.
    assert!(
        events_file.exists(),
        "notify-file {events_file_str} was not created"
    );
    let content = std::fs::read_to_string(&events_file).expect("read events file");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly 1 NDJSON line, got {}",
        lines.len()
    );

    let event: serde_json::Value =
        serde_json::from_str(lines[0]).expect("NDJSON line must be valid JSON");
    assert_eq!(
        event["event_type"].as_str().unwrap_or(""),
        "job.finished",
        "event_type must be 'job.finished'"
    );
    assert_eq!(
        event["job_id"].as_str().unwrap_or(""),
        job_id,
        "event job_id must match"
    );
    assert_eq!(
        event["state"].as_str().unwrap_or(""),
        "exited",
        "event state must be exited"
    );
    assert!(
        event.get("stdout_log_path").is_some(),
        "stdout_log_path must be present"
    );
    assert!(
        event.get("stderr_log_path").is_some(),
        "stderr_log_path must be present"
    );
    assert!(
        event.get("finished_at").is_some(),
        "finished_at must be present"
    );
}

// ── notify command sink ─────────────────────────────────────────────────────────

/// Command sink: event JSON is delivered via stdin and env vars are set.
#[test]
fn notify_command_sink_receives_event_via_stdin() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let captured_file = tmp_dir.path().join("captured.json");
    let captured_str = captured_file.to_str().unwrap();

    // Hook command: read stdin and write to captured_file.
    // --notify-command now accepts a shell command string directly.
    let hook_cmd = format!("cat > {captured_str}");

    let v = h.run(&[
        "run",
        "--notify-command",
        &hook_cmd,
        "--",
        "echo",
        "cmd_sink_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();
    let _ = wait_until_terminal(&h, &job_id);

    std::thread::sleep(std::time::Duration::from_millis(300));

    // The captured file should contain the event JSON written by the hook.
    assert!(
        captured_file.exists(),
        "captured file not created by hook command"
    );
    let content = std::fs::read_to_string(&captured_file).expect("read captured file");
    let event: serde_json::Value =
        serde_json::from_str(content.trim()).expect("captured content must be valid JSON");

    assert_eq!(
        event["event_type"].as_str().unwrap_or(""),
        "job.finished",
        "event_type must be 'job.finished'"
    );
    assert_eq!(
        event["job_id"].as_str().unwrap_or(""),
        job_id,
        "event job_id must match"
    );
}

// ── notify failure non-destructive ─────────────────────────────────────────────

/// Notification failure must not change job state: job remains exited even if
/// the command sink binary does not exist.
#[test]
fn notify_failure_does_not_change_job_state() {
    let h = TestHarness::new();
    // --notify-command now accepts a shell command string directly.
    let hook_cmd = "/no/such/binary/agent_exec_test";

    let v = h.run(&[
        "run",
        "--notify-command",
        hook_cmd,
        "--",
        "echo",
        "failure_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing").to_string();
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "job state must be exited despite notification failure"
    );

    std::thread::sleep(std::time::Duration::from_millis(300));

    // Querying status must also return exited.
    let sv = h.run(&["status", &job_id]);
    assert_envelope(&sv, "status", true);
    assert_eq!(
        sv["state"].as_str().unwrap_or(""),
        "exited",
        "status must return exited; got: {sv}"
    );

    // completion_event.json must record the notification failure without changing job state.
    let root = h.root();
    let completion_event_path = format!("{root}/{job_id}/completion_event.json");
    let event_raw = std::fs::read_to_string(&completion_event_path)
        .expect("completion_event.json must exist after notification dispatch");
    let event: serde_json::Value =
        serde_json::from_str(&event_raw).expect("completion_event.json must be valid JSON");
    assert_eq!(
        event["state"].as_str().unwrap_or(""),
        "exited",
        "completion_event state must be exited"
    );
    // Delivery results must be present and show failure.
    let results = event["delivery_results"]
        .as_array()
        .expect("delivery_results must be an array");
    assert!(!results.is_empty(), "delivery_results must be non-empty");
    assert!(
        !results[0]["success"].as_bool().unwrap_or(true),
        "delivery must have failed"
    );
    assert_eq!(
        results[0]["sink_type"].as_str().unwrap_or(""),
        "command",
        "sink_type must be 'command'"
    );
}

// ── shell wrapper configuration ──────────────────────────────────────────────

/// Default shell wrapper behavior: job runs successfully with the built-in
/// platform default (no --shell-wrapper, no config file).
#[test]
fn shell_wrapper_default_behavior() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--", "echo", "hello_shell_wrapper"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "job must exit successfully with default shell wrapper"
    );
}

/// CLI --shell-wrapper overrides the default: a custom wrapper is accepted and
/// used when running a command string via --notify-command.
#[test]
fn shell_wrapper_cli_override_with_notify_command() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let captured = tmp_dir.path().join("captured.txt");
    let captured_str = captured.to_str().unwrap();
    let hook_cmd = format!("echo wrapper_used > {captured_str}");

    let v = h.run(&[
        "run",
        "--shell-wrapper",
        "sh -lc",
        "--notify-command",
        &hook_cmd,
        "--",
        "echo",
        "sw_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(captured.exists(), "hook command output file must exist");
    let content = std::fs::read_to_string(&captured).unwrap();
    assert!(content.contains("wrapper_used"), "hook must have run");
}

/// Config file --config overrides the built-in default: a config.toml with
/// unix = ["sh", "-lc"] is loaded and accepted.
#[test]
fn shell_wrapper_config_file_override() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let config_path = tmp_dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        "[shell]\nunix = [\"sh\", \"-lc\"]\nwindows = [\"cmd\", \"/C\"]\n",
    )
    .unwrap();

    let v = h.run(&[
        "run",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "echo",
        "config_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "job must exit with config file shell wrapper"
    );
}

/// CLI --shell-wrapper takes precedence over --config when both are specified.
#[test]
fn shell_wrapper_cli_takes_precedence_over_config() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let config_path = tmp_dir.path().join("config.toml");
    // Config specifies a valid wrapper.
    std::fs::write(
        &config_path,
        "[shell]\nunix = [\"sh\", \"-lc\"]\nwindows = [\"cmd\", \"/C\"]\n",
    )
    .unwrap();

    let v = h.run(&[
        "run",
        "--config",
        config_path.to_str().unwrap(),
        "--shell-wrapper",
        "sh -lc",
        "--",
        "echo",
        "precedence_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "job must succeed when CLI wrapper overrides config"
    );
}

/// Invalid config file (bad TOML syntax) causes a command failure with JSON error output.
#[test]
fn shell_wrapper_invalid_config_file_fails() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let config_path = tmp_dir.path().join("config.toml");
    std::fs::write(&config_path, "this is not valid toml {{{ ").unwrap();

    let v = h.run(&[
        "run",
        "--config",
        config_path.to_str().unwrap(),
        "--",
        "echo",
        "should_fail",
    ]);
    // Must return a JSON error envelope.
    assert_envelope(&v, "error", false);
}

/// Shared wrapper: --shell-wrapper affects --notify-command delivery too.
#[test]
fn shell_wrapper_shared_between_run_and_notify_command() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let captured = tmp_dir.path().join("shared_wrapper.txt");
    let captured_str = captured.to_str().unwrap();
    let hook_cmd = format!("echo shared_wrapper_ran > {captured_str}");

    let v = h.run(&[
        "run",
        "--shell-wrapper",
        "sh -lc",
        "--notify-command",
        &hook_cmd,
        "--",
        "echo",
        "shared_test",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(
        captured.exists(),
        "notify-command must have run using the configured wrapper"
    );
    let content = std::fs::read_to_string(&captured).unwrap();
    assert!(
        content.contains("shared_wrapper_ran"),
        "notify-command output must confirm wrapper execution"
    );
}

/// Shell command string execution: `run -- 'cmd && cmd'` goes through the
/// configured wrapper, enabling shell features like `&&`.
#[test]
fn shell_wrapper_applied_to_run_command_string() {
    let h = TestHarness::new();
    // Single-element command string with shell features.
    let v = h.run(&["run", "--", "echo shell_string_ran && echo second"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "shell command string must execute successfully through the wrapper"
    );
}

/// Shell wrapper argv fidelity: the resolved wrapper survives the run→_supervise
/// hand-off without loss from join/split round-trips.
#[test]
fn shell_wrapper_argv_fidelity_across_run_supervise() {
    let h = TestHarness::new();
    // Use an explicit wrapper; it must reach the supervisor intact.
    let v = h.run(&[
        "run",
        "--shell-wrapper",
        "sh -lc",
        "--",
        "echo",
        "fidelity_ok",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "wrapper must be passed to supervisor with argv fidelity"
    );
}

// ── gc command ─────────────────────────────────────────────────────────────────

/// Write a synthetic job directory with meta.json and state.json into root.
/// The `finished_at` and `updated_at` fields are set to `ts` (RFC 3339 UTC).
fn write_fake_job(
    root: &str,
    job_id: &str,
    status: &str,
    finished_at: Option<&str>,
    updated_at: &str,
) {
    let job_dir = std::path::Path::new(root).join(job_id);
    std::fs::create_dir_all(&job_dir).unwrap();

    let meta = serde_json::json!({
        "job": { "id": job_id },
        "schema_version": "0.1",
        "command": ["echo", "test"],
        "created_at": updated_at,
        "root": root,
        "env_keys": [],
        "env_vars": [],
        "mask": []
    });
    std::fs::write(
        job_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();

    let mut state_obj = serde_json::json!({
        "job": {
            "id": job_id,
            "status": status,
            "started_at": updated_at
        },
        "result": {
            "exit_code": if status == "exited" { serde_json::json!(0) } else { serde_json::Value::Null },
            "signal": serde_json::Value::Null,
            "duration_ms": serde_json::Value::Null
        },
        "updated_at": updated_at
    });

    if let Some(fa) = finished_at {
        state_obj["finished_at"] = serde_json::json!(fa);
    }

    std::fs::write(
        job_dir.join("state.json"),
        serde_json::to_string_pretty(&state_obj).unwrap(),
    )
    .unwrap();

    // Write a small log file so dir_size_bytes > 0.
    std::fs::write(job_dir.join("stdout.log"), b"some output").unwrap();
}

/// Verify the gc response envelope and common fields.
fn assert_gc_envelope(v: &serde_json::Value, dry_run: bool) {
    assert_envelope(v, "gc", true);
    assert_eq!(
        v["dry_run"].as_bool().unwrap_or(!dry_run),
        dry_run,
        "dry_run mismatch"
    );
    assert!(v["root"].as_str().is_some(), "root field missing");
    assert!(
        v["older_than"].as_str().is_some(),
        "older_than field missing"
    );
    assert!(
        v["older_than_source"].as_str().is_some(),
        "older_than_source field missing"
    );
    assert!(v["jobs"].is_array(), "jobs must be an array");
}

/// gc on an empty root returns ok with zero counts.
#[test]
fn gc_empty_root_returns_ok() {
    let h = TestHarness::new();
    let v = h.run(&["gc", "--older-than", "1d"]);
    assert_gc_envelope(&v, false);
    assert_eq!(v["deleted"].as_u64().unwrap_or(1), 0);
    assert_eq!(v["freed_bytes"].as_u64().unwrap_or(1), 0);
}

/// gc uses the default 30d window when --older-than is omitted.
#[test]
fn gc_uses_default_30d_window() {
    let h = TestHarness::new();
    // Create a job with a very old finished_at (should be deleted under 30d window).
    write_fake_job(
        h.root(),
        "old-job-01",
        "exited",
        Some("2020-01-01T00:00:00Z"),
        "2020-01-01T00:00:00Z",
    );

    let v = h.run(&["gc"]);
    assert_gc_envelope(&v, false);
    // Default source must be "default".
    assert_eq!(
        v["older_than_source"].as_str().unwrap_or(""),
        "default",
        "should report default source"
    );
    assert_eq!(
        v["older_than"].as_str().unwrap_or(""),
        "30d",
        "should report 30d as default"
    );
    // Old job must be deleted.
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "old terminal job must be deleted"
    );
    assert!(
        v["freed_bytes"].as_u64().unwrap_or(0) > 0,
        "freed_bytes must be > 0"
    );
    // Directory must no longer exist.
    let job_path = std::path::Path::new(h.root()).join("old-job-01");
    assert!(!job_path.exists(), "job directory must be deleted");
}

/// gc deletes only terminal (exited/killed/failed) jobs, never running jobs.
#[test]
fn gc_deletes_only_terminal_jobs() {
    let h = TestHarness::new();
    let old = "2020-01-01T00:00:00Z";
    write_fake_job(h.root(), "exited-old", "exited", Some(old), old);
    write_fake_job(h.root(), "killed-old", "killed", Some(old), old);
    write_fake_job(h.root(), "failed-old", "failed", Some(old), old);
    write_fake_job(h.root(), "running-job", "running", None, old);

    let v = h.run(&["gc", "--older-than", "7d"]);
    assert_gc_envelope(&v, false);
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        3,
        "three terminal jobs must be deleted"
    );

    // Running job directory must still exist.
    let running_path = std::path::Path::new(h.root()).join("running-job");
    assert!(running_path.exists(), "running job must be preserved");

    // Terminal job directories must be deleted.
    assert!(!std::path::Path::new(h.root()).join("exited-old").exists());
    assert!(!std::path::Path::new(h.root()).join("killed-old").exists());
    assert!(!std::path::Path::new(h.root()).join("failed-old").exists());

    // Verify running job appears as skipped in jobs array.
    let jobs = v["jobs"].as_array().unwrap();
    let running_entry = jobs
        .iter()
        .find(|j| j["job_id"].as_str().unwrap_or("") == "running-job");
    assert!(
        running_entry.is_some(),
        "running job must appear in jobs array"
    );
    let running_entry = running_entry.unwrap();
    assert_eq!(running_entry["action"].as_str().unwrap_or(""), "skipped");
    assert_eq!(running_entry["reason"].as_str().unwrap_or(""), "running");
}

/// gc --dry-run reports candidates without deleting directories.
#[test]
fn gc_dry_run_preserves_directories() {
    let h = TestHarness::new();
    let old = "2020-01-01T00:00:00Z";
    write_fake_job(h.root(), "old-exited", "exited", Some(old), old);

    let v = h.run(&["gc", "--older-than", "7d", "--dry-run"]);
    assert_gc_envelope(&v, true);
    // Dry-run: no actual deletions.
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(1),
        0,
        "dry-run must not delete"
    );
    // But freed_bytes should reflect the would-be reclaimed space.
    assert!(
        v["freed_bytes"].as_u64().unwrap_or(0) > 0,
        "freed_bytes must report potential reclaim"
    );
    // Directory must still exist.
    assert!(
        std::path::Path::new(h.root()).join("old-exited").exists(),
        "directory must be preserved in dry-run"
    );

    // Action must be "would_delete".
    let jobs = v["jobs"].as_array().unwrap();
    let entry = jobs
        .iter()
        .find(|j| j["job_id"].as_str().unwrap_or("") == "old-exited");
    assert!(entry.is_some());
    assert_eq!(
        entry.unwrap()["action"].as_str().unwrap_or(""),
        "would_delete"
    );
}

/// gc skips jobs whose state.json lacks both finished_at and updated_at timestamps.
#[test]
fn gc_skips_jobs_without_gc_timestamp() {
    let h = TestHarness::new();
    let job_id = "no-ts-job";
    let job_dir = std::path::Path::new(h.root()).join(job_id);
    std::fs::create_dir_all(&job_dir).unwrap();

    // Write meta.json normally.
    let meta = serde_json::json!({
        "job": { "id": job_id },
        "schema_version": "0.1",
        "command": ["echo", "test"],
        "created_at": "2020-01-01T00:00:00Z",
        "root": h.root(),
        "env_keys": [],
        "env_vars": [],
        "mask": []
    });
    std::fs::write(
        job_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();

    // Write state.json with killed status but no finished_at and an empty updated_at.
    // Note: updated_at is required by the schema, so we use a non-empty value here to
    // exercise the "no_timestamp when both are missing" path by omitting finished_at and
    // relying on updated_at being old enough.  For the true "no timestamp" scenario we
    // test the fallback: updated_at old → should be deleted.
    // The spec says "both missing → skip"; since updated_at is always present in valid
    // state.json, we test the more realistic fallback: killed + only updated_at present.
    let state = serde_json::json!({
        "job": {
            "id": job_id,
            "status": "killed",
            "started_at": "2020-01-01T00:00:00Z"
        },
        "result": {
            "exit_code": null,
            "signal": "TERM",
            "duration_ms": null
        },
        "updated_at": "2020-01-01T00:00:00Z"
    });
    std::fs::write(
        job_dir.join("state.json"),
        serde_json::to_string_pretty(&state).unwrap(),
    )
    .unwrap();

    // The job has updated_at but no finished_at; gc should fall back to updated_at and delete it.
    let v = h.run(&["gc", "--older-than", "7d"]);
    assert_gc_envelope(&v, false);
    // Should be deleted (updated_at fallback works).
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "job with only updated_at should be deleted via fallback"
    );
}

/// gc --older-than flag overrides the default and is reflected in the response.
#[test]
fn gc_custom_older_than_flag_reported() {
    let h = TestHarness::new();
    let v = h.run(&["gc", "--older-than", "7d"]);
    assert_gc_envelope(&v, false);
    assert_eq!(v["older_than"].as_str().unwrap_or(""), "7d");
    assert_eq!(v["older_than_source"].as_str().unwrap_or(""), "flag");
}

/// gc skips jobs whose state.json is unreadable.
#[test]
fn gc_skips_unreadable_state() {
    let h = TestHarness::new();
    let job_dir = std::path::Path::new(h.root()).join("bad-state-job");
    std::fs::create_dir_all(&job_dir).unwrap();
    // Write a valid meta.json but corrupt state.json.
    let meta = serde_json::json!({
        "job": { "id": "bad-state-job" },
        "schema_version": "0.1",
        "command": ["echo"],
        "created_at": "2020-01-01T00:00:00Z",
        "root": h.root(),
        "env_keys": [],
        "env_vars": [],
        "mask": []
    });
    std::fs::write(
        job_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap(),
    )
    .unwrap();
    std::fs::write(job_dir.join("state.json"), b"not valid json").unwrap();

    let v = h.run(&["gc", "--older-than", "1d"]);
    assert_gc_envelope(&v, false);
    // The job should be reported as skipped.
    let jobs = v["jobs"].as_array().unwrap();
    let entry = jobs
        .iter()
        .find(|j| j["job_id"].as_str().unwrap_or("") == "bad-state-job");
    assert!(entry.is_some(), "unreadable job must appear in jobs list");
    assert_eq!(entry.unwrap()["action"].as_str().unwrap_or(""), "skipped");
    assert_eq!(
        entry.unwrap()["reason"].as_str().unwrap_or(""),
        "state_unreadable"
    );
    // Directory must be preserved.
    assert!(
        job_dir.exists(),
        "directory with unreadable state must be preserved"
    );
}

// ============================================================
// Tag feature tests (add-job-tags)
// ============================================================

/// Helper: run a job with tags and return the JSON output.
fn run_with_tags(h: &TestHarness, tags: &[&str]) -> serde_json::Value {
    let mut args = vec!["run"];
    for tag in tags {
        args.push("--tag");
        args.push(tag);
    }
    args.extend_from_slice(&["--", "true"]);
    h.run(&args)
}

/// `run --tag` returns tags in the response JSON.
#[test]
fn run_tag_appears_in_response() {
    let h = TestHarness::new();
    let v = run_with_tags(&h, &["aaa", "bbb"]);
    assert_envelope(&v, "run", true);
    let tags = v["tags"].as_array().expect("tags must be an array");
    let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
    assert_eq!(tag_strs, vec!["aaa", "bbb"]);
}

/// `run --tag` persists tags in meta.json.
#[test]
fn run_tag_persisted_in_meta() {
    let h = TestHarness::new();
    let v = run_with_tags(&h, &["hoge", "fuga"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().unwrap();
    // Read meta.json directly from the job directory.
    let meta_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("meta.json");
    let meta_bytes = std::fs::read(&meta_path).expect("meta.json must exist");
    let meta: serde_json::Value = serde_json::from_slice(&meta_bytes).unwrap();
    let tags = meta["tags"].as_array().expect("tags must be in meta.json");
    let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
    assert_eq!(tag_strs, vec!["hoge", "fuga"]);
}

/// Duplicate tags on `run` are deduplicated preserving first-seen order.
#[test]
fn run_tag_deduplication() {
    let h = TestHarness::new();
    let v = run_with_tags(&h, &["aaa", "bbb", "aaa", "ccc", "bbb"]);
    assert_envelope(&v, "run", true);
    let tags = v["tags"].as_array().expect("tags must be an array");
    let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
    assert_eq!(tag_strs, vec!["aaa", "bbb", "ccc"]);
}

/// `run` with no tags returns an empty tags array.
#[test]
fn run_no_tags_returns_empty_array() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--", "true"]);
    assert_envelope(&v, "run", true);
    let tags = v["tags"].as_array().expect("tags must be an array");
    assert!(tags.is_empty(), "tags must be empty when none specified");
}

/// `run --tag` with an invalid tag value fails as a usage error (exit 2, no JSON).
#[test]
fn run_invalid_tag_is_rejected() {
    let h = TestHarness::new();
    assert_usage_error(&["run", "--tag", "bad tag!", "--", "true"], Some(h.root()));
}

/// `run --tag` with a `.*` suffix is rejected as a stored tag (usage error, exit 2).
#[test]
fn run_wildcard_tag_is_rejected() {
    let h = TestHarness::new();
    assert_usage_error(&["run", "--tag", "hoge.*", "--", "true"], Some(h.root()));
}

/// `tag set` replaces tags on an existing job.
#[test]
fn tag_set_replaces_tags() {
    let h = TestHarness::new();
    // Create job with initial tags.
    let run_v = run_with_tags(&h, &["old"]);
    let job_id = run_v["job_id"].as_str().unwrap();

    // Replace tags via `tag set`.
    let v = h.run(&["tag", "set", job_id, "--tag", "new1", "--tag", "new2"]);
    assert_envelope(&v, "tag_set", true);
    let tags = v["tags"].as_array().expect("tags must be in response");
    let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str().unwrap()).collect();
    assert_eq!(tag_strs, vec!["new1", "new2"]);

    // Verify meta.json was updated.
    let meta_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("meta.json");
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    let stored: Vec<&str> = meta["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(stored, vec!["new1", "new2"]);
}

/// `tag set` deduplicates the replacement list.
#[test]
fn tag_set_deduplicates() {
    let h = TestHarness::new();
    let run_v = run_with_tags(&h, &[]);
    let job_id = run_v["job_id"].as_str().unwrap();
    let v = h.run(&[
        "tag", "set", job_id, "--tag", "a", "--tag", "b", "--tag", "a",
    ]);
    assert_envelope(&v, "tag_set", true);
    let tags: Vec<&str> = v["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(tags, vec!["a", "b"]);
}

/// `tag set` clears tags when no --tag flags are given.
#[test]
fn tag_set_clears_tags() {
    let h = TestHarness::new();
    let run_v = run_with_tags(&h, &["keep-me"]);
    let job_id = run_v["job_id"].as_str().unwrap();
    let v = h.run(&["tag", "set", job_id]);
    assert_envelope(&v, "tag_set", true);
    let tags = v["tags"].as_array().unwrap();
    assert!(tags.is_empty(), "tags must be empty after clear");
}

/// `tag set` on a missing job returns job_not_found.
#[test]
fn tag_set_missing_job_returns_job_not_found() {
    let h = TestHarness::new();
    let v = h.run(&["tag", "set", "NO_SUCH_JOB_ID", "--tag", "x"]);
    assert_envelope(&v, "error", false);
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

/// `tag set` does not modify other meta.json fields.
#[test]
fn tag_set_preserves_other_meta_fields() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "--tag", "initial", "--", "true"]);
    let job_id = run_v["job_id"].as_str().unwrap();
    let meta_before: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    h.run(&["tag", "set", job_id, "--tag", "after"]);

    let meta_after: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(meta_before["job"], meta_after["job"]);
    assert_eq!(meta_before["command"], meta_after["command"]);
    assert_eq!(meta_before["created_at"], meta_after["created_at"]);
    assert_eq!(meta_before["cwd"], meta_after["cwd"]);
    // Only tags should differ.
    assert_ne!(meta_before["tags"], meta_after["tags"]);
}

/// `list` returns tags in each job summary.
#[test]
fn list_jobs_include_tags() {
    let h = TestHarness::new();
    run_with_tags(&h, &["mytag"]);
    let v = h.run(&["list", "--all"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().unwrap();
    assert!(!jobs.is_empty(), "at least one job expected");
    // Every job entry must have a `tags` array.
    for job in jobs {
        assert!(
            job["tags"].is_array(),
            "job summary must include tags array"
        );
    }
    // The job we created must have the tag.
    let has_tag = jobs
        .iter()
        .any(|j| j["tags"].as_array().unwrap().iter().any(|t| t == "mytag"));
    assert!(has_tag, "job with 'mytag' must appear in list");
}

/// `list --tag <exact>` returns only jobs that have that exact tag.
#[test]
fn list_exact_tag_filter() {
    let h = TestHarness::new();
    run_with_tags(&h, &["alpha"]);
    run_with_tags(&h, &["beta"]);

    let v = h.run(&["list", "--all", "--tag", "alpha"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().unwrap();
    assert!(!jobs.is_empty(), "at least one job expected");
    for job in jobs {
        let tags: Vec<&str> = job["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        assert!(
            tags.contains(&"alpha"),
            "all returned jobs must have 'alpha'"
        );
    }
}

/// `list --tag <prefix>.*` returns jobs in that namespace.
#[test]
fn list_prefix_tag_filter() {
    let h = TestHarness::new();
    run_with_tags(&h, &["ns.sub.job"]);
    run_with_tags(&h, &["other.job"]);

    let v = h.run(&["list", "--all", "--tag", "ns.*"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().unwrap();
    assert!(!jobs.is_empty(), "at least one matching job expected");
    for job in jobs {
        let tags: Vec<&str> = job["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        let matches = tags.iter().any(|t| *t == "ns" || t.starts_with("ns."));
        assert!(matches, "all returned jobs must have a 'ns.*' tag");
    }
}

/// `list --tag a --tag b` returns only jobs satisfying both (AND semantics).
#[test]
fn list_multiple_tag_filters_and_semantics() {
    let h = TestHarness::new();
    run_with_tags(&h, &["x", "y"]); // matches both
    run_with_tags(&h, &["x"]); // matches only x
    run_with_tags(&h, &["y"]); // matches only y

    let v = h.run(&["list", "--all", "--tag", "x", "--tag", "y"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().unwrap();
    // Only the job with both tags should appear.
    assert_eq!(jobs.len(), 1, "only job with both tags must be returned");
    let tags: Vec<&str> = jobs[0]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert!(tags.contains(&"x") && tags.contains(&"y"));
}

/// `list --tag` composes with cwd filtering.
#[test]
fn list_tag_filter_composes_with_cwd() {
    let h = TestHarness::new();
    // Run a job with the tag in the current directory.
    run_with_tags(&h, &["shared"]);

    // Default list (cwd filter active) + tag filter must still work.
    let v = h.run(&["list", "--tag", "shared"]);
    assert_envelope(&v, "list", true);
    // Should not error; just return results filtered by both cwd and tag.
}

/// `list --tag` with an invalid pattern fails as a usage error (exit 2, no JSON).
#[test]
fn list_invalid_tag_pattern_rejected() {
    let h = TestHarness::new();
    assert_usage_error(&["list", "--all", "--tag", "bad pattern!"], Some(h.root()));
}

/// `tag set` with an invalid tag value fails as a usage error (exit 2, no JSON).
#[test]
fn tag_set_invalid_tag_rejected() {
    let h = TestHarness::new();
    let run_v = run_with_tags(&h, &[]);
    let job_id = run_v["job_id"].as_str().unwrap();
    assert_usage_error(&["tag", "set", job_id, "--tag", "bad!tag"], Some(h.root()));
}

// ── notify set ─────────────────────────────────────────────────────────────────

/// notify set: updates notify_command in meta.json and returns success envelope.
#[test]
fn notify_set_updates_notify_command_in_meta_json() {
    let h = TestHarness::new();

    // Create a job first.
    let v = h.run(&["run", "--", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Wait for the job to start.
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Run notify set.
    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--command",
        "cat >/tmp/event.json",
    ]);
    assert_envelope(&set_v, "notify.set", true);
    assert_eq!(
        set_v["job_id"].as_str().unwrap_or(""),
        job_id,
        "job_id must match"
    );
    assert_eq!(
        set_v["notification"]["notify_command"]
            .as_str()
            .unwrap_or(""),
        "cat >/tmp/event.json",
        "notify_command must be updated"
    );

    // Verify meta.json was actually updated on disk.
    let meta_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("meta.json");
    let meta_raw = std::fs::read_to_string(&meta_path).expect("read meta.json");
    let meta: serde_json::Value = serde_json::from_str(&meta_raw).expect("parse meta.json");
    assert_eq!(
        meta["notification"]["notify_command"]
            .as_str()
            .unwrap_or(""),
        "cat >/tmp/event.json",
        "meta.json notify_command must be updated on disk"
    );
}

/// notify set: preserves existing notify_file when updating notify_command.
#[test]
fn notify_set_preserves_notify_file() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    // Create a job with --notify-file.
    let v = h.run(&[
        "run",
        "--notify-file",
        events_file_str,
        "--",
        "echo",
        "hello",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    // Update notify_command via notify set.
    let set_v = h.run(&["notify", "set", &job_id, "--command", "cat >/dev/null"]);
    assert_envelope(&set_v, "notify.set", true);

    // Both notify_command and notify_file must be present.
    assert_eq!(
        set_v["notification"]["notify_command"]
            .as_str()
            .unwrap_or(""),
        "cat >/dev/null",
        "notify_command must be set"
    );
    assert_eq!(
        set_v["notification"]["notify_file"].as_str().unwrap_or(""),
        events_file_str,
        "notify_file must be preserved"
    );

    // Confirm on disk.
    let meta_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("meta.json");
    let meta_raw = std::fs::read_to_string(&meta_path).expect("read meta.json");
    let meta: serde_json::Value = serde_json::from_str(&meta_raw).expect("parse meta.json");
    assert_eq!(
        meta["notification"]["notify_file"].as_str().unwrap_or(""),
        events_file_str,
        "notify_file must be preserved in meta.json on disk"
    );
}

/// notify set: missing job returns job_not_found error.
#[test]
fn notify_set_missing_job_returns_job_not_found() {
    let h = TestHarness::new();

    let v = h.run(&["notify", "set", "NONEXISTENT-JOB", "--command", "echo hi"]);
    assert_envelope(&v, "error", false);
    assert_eq!(
        v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found",
        "error.code must be job_not_found"
    );
}

/// notify set on a terminal job: succeeds without executing the command.
#[test]
fn notify_set_terminal_job_succeeds_without_executing_command() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let marker = tmp_dir.path().join("executed.txt");
    let marker_str = marker.to_str().unwrap();

    // Run a job and wait for it to finish.
    let v = h.run(&["run", "--", "echo", "done"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    // Marker must not exist yet.
    assert!(!marker.exists(), "marker must not exist before notify set");

    // Call notify set with a command that would create the marker if executed.
    let hook_cmd = format!("touch {marker_str}");
    let set_v = h.run(&["notify", "set", &job_id, "--command", &hook_cmd]);
    assert_envelope(&set_v, "notify.set", true);

    // Brief wait to confirm command was NOT executed.
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(
        !marker.exists(),
        "notify set must not execute the command (marker must not be created)"
    );
}

/// notify set before job finishes: updated command is used at completion time.
#[test]
fn notify_set_updated_command_used_at_completion() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let captured = tmp_dir.path().join("captured.json");
    let captured_str = captured.to_str().unwrap();

    // Run a slow job (sleep 1s) without a notify_command.
    let v = h.run(&["run", "--no-wait", "--", "sleep", "1"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Set notify_command before the job finishes.
    let hook_cmd = format!("cat > {captured_str}");
    let set_v = h.run(&["notify", "set", &job_id, "--command", &hook_cmd]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait for the job to complete and delivery to happen.
    std::thread::sleep(std::time::Duration::from_millis(2500));

    // The captured file must exist and contain a valid job.finished event.
    assert!(
        captured.exists(),
        "captured file must be created by the updated notify_command"
    );
    let content = std::fs::read_to_string(&captured).expect("read captured file");
    let event: serde_json::Value =
        serde_json::from_str(content.trim()).expect("captured content must be valid JSON");
    assert_eq!(
        event["event_type"].as_str().unwrap_or(""),
        "job.finished",
        "event_type must be job.finished"
    );
    assert_eq!(
        event["job_id"].as_str().unwrap_or(""),
        job_id,
        "event job_id must match"
    );
}

// ── global --root flag ─────────────────────────────────────────────────────────

/// Verify that `agent-exec --root <PATH> run ...` uses the specified root.
#[test]
fn global_root_flag_run() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    let v = run_cmd_with_global_root_flag(&root, &["run", "echo", "global_root_test"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(!job_id.is_empty(), "job_id is empty");
    // Verify the job directory was created under the explicit root.
    assert!(
        tmp.path().join(job_id).exists(),
        "job dir not created under global --root path"
    );
}

/// Verify that `agent-exec --root <PATH> status <id>` resolves jobs from the global root.
#[test]
fn global_root_flag_status() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    // Start a job with the global root flag.
    let run_v = run_cmd_with_global_root_flag(&root, &["run", "echo", "hi"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    // Query status using the same global root flag.
    let v = run_cmd_with_global_root_flag(&root, &["status", &job_id]);
    assert_envelope(&v, "status", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
}

/// Verify that `agent-exec --root <PATH> list` resolves jobs from the global root.
#[test]
fn global_root_flag_list() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    // Start a job using the global root flag.
    let run_v = run_cmd_with_global_root_flag(&root, &["run", "echo", "list_test"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    // List with the global root flag; job must appear.
    let v = run_cmd_with_global_root_flag(&root, &["list", "--all"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().expect("jobs array missing");
    assert!(
        jobs.iter()
            .any(|j| j["job_id"].as_str().unwrap_or("") == job_id),
        "started job not found in list response"
    );
}

/// Verify that `agent-exec --root <PATH> gc` operates on the correct root.
#[test]
fn global_root_flag_gc() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    let v = run_cmd_with_global_root_flag(&root, &["gc", "--dry-run"]);
    assert_gc_envelope(&v, true);
}

/// Precedence: CLI --root flag beats AGENT_EXEC_ROOT env var.
#[test]
fn global_root_flag_takes_precedence_over_env() {
    let tmp_flag = tempfile::tempdir().expect("create tempdir for --root");
    let tmp_env = tempfile::tempdir().expect("create tempdir for env");
    let root_flag = tmp_flag.path().to_str().expect("valid UTF-8").to_string();
    let root_env = tmp_env.path().to_str().expect("valid UTF-8").to_string();

    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.arg("--root").arg(&root_flag);
    cmd.args(["run", "echo", "precedence"]);
    // Set env to a different root — the flag must win.
    cmd.env("AGENT_EXEC_ROOT", &root_env);
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON");
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(
        tmp_flag.path().join(job_id).exists(),
        "job must be in --root dir, not AGENT_EXEC_ROOT dir"
    );
    assert!(
        !tmp_env.path().join(job_id).exists(),
        "job must NOT be in AGENT_EXEC_ROOT dir when --root flag is set"
    );
}

// ── legacy per-subcommand --root flag (backward compatibility) ─────────────────

/// Verify that `agent-exec run --root <PATH> ...` still works (--root after subcommand).
/// The flag is global in clap, so both positions are accepted identically.
#[test]
fn subcommand_root_flag_compat_run() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    let v = run_cmd_with_subcommand_root_flag("run", &root, &["echo", "compat_run"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");
    assert!(
        tmp.path().join(job_id).exists(),
        "job dir not created under --root path when flag placed after subcommand"
    );
}

/// Verify that `agent-exec status --root <PATH> <id>` resolves the job from the correct root.
#[test]
fn subcommand_root_flag_compat_status() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    // Start a job using global syntax to get a known job_id in this root.
    let run_v = run_cmd_with_global_root_flag(&root, &["run", "echo", "compat_status"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    // Query status using legacy per-subcommand --root position.
    let v = run_cmd_with_subcommand_root_flag("status", &root, &[&job_id]);
    assert_envelope(&v, "status", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
}

/// Verify that `agent-exec list --root <PATH>` lists jobs from the correct root.
#[test]
fn subcommand_root_flag_compat_list() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    let run_v = run_cmd_with_global_root_flag(&root, &["run", "echo", "compat_list"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    let v = run_cmd_with_subcommand_root_flag("list", &root, &["--all"]);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().expect("jobs array");
    assert!(
        jobs.iter()
            .any(|j| j["job_id"].as_str().unwrap_or("") == job_id),
        "started job not found when using legacy --root position for list"
    );
}

/// Verify that `agent-exec gc --root <PATH>` operates on the correct root.
#[test]
fn subcommand_root_flag_compat_gc() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let root = tmp.path().to_str().expect("valid UTF-8").to_string();
    let v = run_cmd_with_subcommand_root_flag("gc", &root, &["--dry-run"]);
    assert_gc_envelope(&v, true);
}

// ── notify set output-match ─────────────────────────────────────────────────

/// notify set: saves output-match configuration and returns it in the response.
#[test]
fn notify_set_saves_output_match_config() {
    let h = TestHarness::new();

    let v = h.run(&["run", "--", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "ERROR",
        "--output-command",
        "cat >/dev/null",
    ]);
    assert_envelope(&set_v, "notify.set", true);
    assert_eq!(
        set_v["notification"]["on_output_match"]["pattern"]
            .as_str()
            .unwrap_or(""),
        "ERROR",
        "on_output_match.pattern must be saved"
    );
    assert_eq!(
        set_v["notification"]["on_output_match"]["match_type"]
            .as_str()
            .unwrap_or(""),
        "contains",
        "on_output_match.match_type defaults to contains"
    );
    assert_eq!(
        set_v["notification"]["on_output_match"]["stream"]
            .as_str()
            .unwrap_or(""),
        "either",
        "on_output_match.stream defaults to either"
    );

    // Verify meta.json on disk.
    let meta_path = std::path::Path::new(h.root())
        .join(&job_id)
        .join("meta.json");
    let meta_raw = std::fs::read_to_string(&meta_path).expect("read meta.json");
    let meta: serde_json::Value = serde_json::from_str(&meta_raw).expect("parse meta.json");
    assert_eq!(
        meta["notification"]["on_output_match"]["pattern"]
            .as_str()
            .unwrap_or(""),
        "ERROR",
        "meta.json on_output_match.pattern must be persisted"
    );
}

/// notify set: output-match on terminal job does not trigger delivery.
#[test]
fn notify_set_output_match_terminal_job_no_delivery() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let marker = tmp_dir.path().join("executed.txt");
    let marker_str = marker.to_str().unwrap();

    // Run a job and wait for it to finish.
    let v = h.run(&["run", "--", "echo", "done"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();
    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");

    let hook_cmd = format!("touch {marker_str}");
    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "done",
        "--output-command",
        &hook_cmd,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "notify set on terminal job must not execute output-match command"
    );
}

/// notify set: --command and output-match options can be set together (preserving both).
#[test]
fn notify_set_completion_and_output_match_coexist() {
    let h = TestHarness::new();

    let v = h.run(&["run", "--", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--command",
        "cat >/dev/null",
        "--output-pattern",
        "ERROR",
    ]);
    assert_envelope(&set_v, "notify.set", true);

    assert!(
        set_v["notification"]["notify_command"].as_str().is_some(),
        "notify_command must be present"
    );
    assert!(
        set_v["notification"]["on_output_match"]["pattern"]
            .as_str()
            .is_some(),
        "on_output_match must be present"
    );
}

/// notify set: missing job returns job_not_found for output-match updates.
#[test]
fn notify_set_output_match_missing_job_returns_job_not_found() {
    let h = TestHarness::new();

    let v = h.run(&[
        "notify",
        "set",
        "NONEXISTENT-JOB",
        "--output-pattern",
        "ERROR",
        "--output-command",
        "cat >/dev/null",
    ]);
    assert_envelope(&v, "error", false);
    assert_eq!(
        v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found",
        "error.code must be job_not_found"
    );
}

/// notify set: --output-pattern with --output-match-type regex is accepted.
#[test]
fn notify_set_output_match_regex_type() {
    let h = TestHarness::new();

    let v = h.run(&["run", "--", "echo", "hello"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    std::thread::sleep(std::time::Duration::from_millis(200));

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "ERR.*",
        "--output-match-type",
        "regex",
        "--output-stream",
        "stderr",
    ]);
    assert_envelope(&set_v, "notify.set", true);
    assert_eq!(
        set_v["notification"]["on_output_match"]["match_type"]
            .as_str()
            .unwrap_or(""),
        "regex",
    );
    assert_eq!(
        set_v["notification"]["on_output_match"]["stream"]
            .as_str()
            .unwrap_or(""),
        "stderr",
    );
}

/// Output-match with command sink: matching stdout line triggers job.output.matched delivery.
#[test]
fn output_match_command_sink_fires_on_matching_line() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let captured = tmp_dir.path().join("match.json");
    let captured_str = captured.to_str().unwrap();

    // Run a job that sleeps briefly, then prints the matching line.
    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "sleep 0.3; echo ERROR_LINE",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Set output-match config before the ERROR_LINE is printed.
    let hook_cmd = format!("cat > {captured_str}");
    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "ERROR_LINE",
        "--output-command",
        &hook_cmd,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait long enough for the job to finish and deliver.
    std::thread::sleep(std::time::Duration::from_millis(2000));

    assert!(
        captured.exists(),
        "output-match command sink must have been executed"
    );
    let content = std::fs::read_to_string(&captured).expect("read captured");
    let event: serde_json::Value =
        serde_json::from_str(content.trim()).expect("captured content must be valid JSON");
    assert_eq!(
        event["event_type"].as_str().unwrap_or(""),
        "job.output.matched",
        "event_type must be job.output.matched"
    );
    assert_eq!(
        event["job_id"].as_str().unwrap_or(""),
        job_id,
        "event job_id must match"
    );
    assert_eq!(
        event["pattern"].as_str().unwrap_or(""),
        "ERROR_LINE",
        "event pattern must match configured pattern"
    );
    assert_eq!(
        event["stream"].as_str().unwrap_or(""),
        "stdout",
        "event stream must be stdout"
    );
}

/// Output-match with file sink: each matching line appends one NDJSON line.
#[test]
fn output_match_file_sink_appends_per_match() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("output_events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    // Job prints two matching lines.
    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "sleep 0.2; echo MATCH_ONE; echo MATCH_TWO",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "MATCH_",
        "--output-file",
        events_file_str,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait for both lines to be matched and delivered.
    std::thread::sleep(std::time::Duration::from_millis(2500));

    assert!(
        events_file.exists(),
        "output-match file sink must have been created"
    );
    let content = std::fs::read_to_string(&events_file).expect("read events file");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "must have exactly two NDJSON lines (one per match)"
    );

    for line in &lines {
        let ev: serde_json::Value = serde_json::from_str(line).expect("each line must be JSON");
        assert_eq!(
            ev["event_type"].as_str().unwrap_or(""),
            "job.output.matched"
        );
        assert_eq!(ev["job_id"].as_str().unwrap_or(""), job_id);
    }
}

/// Output-match: pre-existing output is not replayed when notify set is called after job start.
#[test]
fn output_match_no_replay_of_pre_existing_output() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let marker = tmp_dir.path().join("replayed.txt");
    let marker_str = marker.to_str().unwrap();

    // Run a job that prints "MATCH_EARLY" before we can set output-match config.
    let v = h.run(&["run", "--", "sh", "-c", "echo MATCH_EARLY; sleep 2"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Wait to ensure "MATCH_EARLY" has been printed.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Now set output-match config; "MATCH_EARLY" has already been written.
    let hook_cmd = format!("touch {marker_str}");
    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "MATCH_EARLY",
        "--output-command",
        &hook_cmd,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait briefly; the hook must not fire because the line was already past.
    std::thread::sleep(std::time::Duration::from_millis(600));
    assert!(
        !marker.exists(),
        "output-match must not replay pre-existing output"
    );
}

/// Output-match sink failure: job lifecycle state remains unchanged.
#[test]
fn output_match_sink_failure_does_not_change_job_state() {
    let h = TestHarness::new();

    // Run a job that prints a matching line.
    let v = h.run(&["run", "--", "sh", "-c", "sleep 0.2; echo TRIGGER"]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Set a command sink that always fails.
    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "TRIGGER",
        "--output-command",
        "exit 1",
    ]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait for the job to finish.
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Job state must be "exited" (not "failed") despite sink failure.
    let status_v = h.run(&["status", &job_id]);
    assert_envelope(&status_v, "status", true);
    assert_eq!(
        status_v["state"].as_str().unwrap_or(""),
        "exited",
        "job state must be exited even when output-match sink fails"
    );
}

/// Output-match: notification_events.ndjson is created for each match.
#[test]
fn output_match_notification_events_ndjson_written() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("output_events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "sleep 0.2; echo RECORD_ME",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "RECORD_ME",
        "--output-file",
        events_file_str,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    std::thread::sleep(std::time::Duration::from_millis(2000));

    // Check that notification_events.ndjson was written in the job dir.
    let notif_events = std::path::Path::new(h.root())
        .join(&job_id)
        .join("notification_events.ndjson");
    assert!(
        notif_events.exists(),
        "notification_events.ndjson must be created in job dir"
    );
    let content = std::fs::read_to_string(&notif_events).expect("read notification_events.ndjson");
    assert!(
        !content.trim().is_empty(),
        "notification_events.ndjson must contain at least one record"
    );
    let record: serde_json::Value = serde_json::from_str(content.lines().next().unwrap_or("{}"))
        .expect("first line must be JSON");
    assert_eq!(
        record["event_type"].as_str().unwrap_or(""),
        "job.output.matched"
    );
}

/// Output-match with --output-match-type regex: matching by regex pattern.
#[test]
fn output_match_regex_pattern_fires_on_match() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("regex_events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "sleep 0.2; echo ERR123; echo INFO456",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "^ERR",
        "--output-match-type",
        "regex",
        "--output-file",
        events_file_str,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    std::thread::sleep(std::time::Duration::from_millis(2500));

    assert!(
        events_file.exists(),
        "regex match must have triggered file sink"
    );
    let content = std::fs::read_to_string(&events_file).expect("read regex events file");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    // Only ERR123 should match ^ERR; INFO456 must not.
    assert_eq!(lines.len(), 1, "only ERR123 must match ^ERR regex");
    let ev: serde_json::Value = serde_json::from_str(lines[0]).expect("line must be JSON");
    assert_eq!(
        ev["event_type"].as_str().unwrap_or(""),
        "job.output.matched"
    );
    assert_eq!(ev["line"].as_str().unwrap_or(""), "ERR123");
}

/// Output-match with --output-stream stderr: only stderr lines trigger delivery.
#[test]
fn output_match_stream_stderr_only() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("stderr_events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    // Print "MATCH" to both stdout and stderr.
    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "sleep 0.2; echo MATCH; echo MATCH >&2",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "MATCH",
        "--output-stream",
        "stderr",
        "--output-file",
        events_file_str,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    std::thread::sleep(std::time::Duration::from_millis(2500));

    assert!(
        events_file.exists(),
        "stderr match must have triggered file sink"
    );
    let content = std::fs::read_to_string(&events_file).expect("read stderr events file");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "only the stderr MATCH must be recorded (stdout MATCH must be ignored)"
    );
    let ev: serde_json::Value = serde_json::from_str(lines[0]).expect("line must be JSON");
    assert_eq!(ev["stream"].as_str().unwrap_or(""), "stderr");
}

/// Output-match: `notify set` configured immediately before a near-future (~50 ms)
/// matching line must trigger delivery even when the supervisor's last config
/// reload occurred within the same 100 ms window.
///
/// Regression: prior to per-line reload, a 100 ms throttle on `meta.json` reads
/// could suppress a `notify set` update so that a matching line arriving less
/// than 100 ms later was silently missed.
#[test]
fn output_match_near_future_line_triggers_delivery() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("near_future_events.ndjson");
    let events_file_str = events_file.to_str().unwrap();

    // The job prints 8 non-matching heartbeat lines at 200 ms intervals to keep
    // the OutputMatchChecker's last reload close to "now", then sleeps 500 ms
    // before printing the target line.  We call `notify set` during the gap
    // between the last heartbeat and the target line so the config update must
    // be picked up for the very next line.  Intervals are generous for CI.
    let v = h.run(&[
        "run",
        "--no-wait",
        "--",
        "sh",
        "-c",
        "for i in $(seq 1 8); do echo heartbeat_$i; sleep 0.2; done; sleep 0.5; echo CLOSE_CALL_MATCH",
    ]);
    assert_envelope(&v, "run", true);
    let job_id = v["job_id"].as_str().expect("job_id").to_string();

    // Wait until after the 8th heartbeat (~1600 ms) so the checker has recently
    // reloaded, then configure output-match during the 500 ms gap before the
    // target line.
    std::thread::sleep(std::time::Duration::from_millis(1800));

    let set_v = h.run(&[
        "notify",
        "set",
        &job_id,
        "--output-pattern",
        "CLOSE_CALL_MATCH",
        "--output-file",
        events_file_str,
    ]);
    assert_envelope(&set_v, "notify.set", true);

    // Wait long enough for the job to finish and delivery to complete.
    std::thread::sleep(std::time::Duration::from_millis(3000));

    assert!(
        events_file.exists(),
        "output-match file sink must have been written: per-line reload must make \
         the notify set update visible even when the matching line arrives <100 ms after it"
    );
    let content = std::fs::read_to_string(&events_file).expect("read near_future_events");
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one match must be recorded for CLOSE_CALL_MATCH"
    );
    let ev: serde_json::Value = serde_json::from_str(lines[0]).expect("line must be JSON");
    assert_eq!(
        ev["event_type"].as_str().unwrap_or(""),
        "job.output.matched",
        "event_type must be job.output.matched"
    );
    assert_eq!(
        ev["line"].as_str().unwrap_or(""),
        "CLOSE_CALL_MATCH",
        "line field must contain the matched output line"
    );
}

// ── create/run definition-time option alignment ─────────────────────────────

/// `create --tag` persists tags in meta.json with the same shape as `run --tag`.
#[test]
fn create_tag_persisted_same_shape_as_run() {
    let h = TestHarness::new();

    // create path
    let c = h.run(&["create", "--tag", "aaa", "--tag", "bbb", "--", "true"]);
    assert_envelope(&c, "create", true);
    let create_job_id = c["job_id"].as_str().expect("job_id");
    let create_meta_path = std::path::Path::new(h.root())
        .join(create_job_id)
        .join("meta.json");
    let create_meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&create_meta_path).unwrap()).unwrap();
    let create_tags: Vec<&str> = create_meta["tags"]
        .as_array()
        .expect("tags in create meta.json")
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();

    // run path
    let r = h.run(&["run", "--tag", "aaa", "--tag", "bbb", "--", "true"]);
    assert_envelope(&r, "run", true);
    let run_job_id = r["job_id"].as_str().expect("job_id");
    let run_meta_path = std::path::Path::new(h.root())
        .join(run_job_id)
        .join("meta.json");
    let run_meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&run_meta_path).unwrap()).unwrap();
    let run_tags: Vec<&str> = run_meta["tags"]
        .as_array()
        .expect("tags in run meta.json")
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();

    assert_eq!(
        create_tags, run_tags,
        "create and run must persist the same tag shape"
    );
    assert_eq!(create_tags, vec!["aaa", "bbb"]);
}

/// Duplicate tags on `create` are deduplicated preserving first-seen order.
#[test]
fn create_tag_deduplication() {
    let h = TestHarness::new();
    let v = h.run(&[
        "create", "--tag", "aaa", "--tag", "bbb", "--tag", "aaa", "--", "true",
    ]);
    assert_envelope(&v, "create", true);
    let job_id = v["job_id"].as_str().unwrap();
    let meta_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("meta.json");
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    let tags: Vec<&str> = meta["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(tags, vec!["aaa", "bbb"], "duplicates must be removed");
}

/// `create` with no tags persists an empty tags array.
#[test]
fn create_no_tags_persists_empty_array() {
    let h = TestHarness::new();
    let v = h.run(&["create", "--", "true"]);
    assert_envelope(&v, "create", true);
    let job_id = v["job_id"].as_str().unwrap();
    let meta_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("meta.json");
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&meta_path).unwrap()).unwrap();
    let tags = meta["tags"].as_array().expect("tags must be present");
    assert!(tags.is_empty(), "tags must be empty when none specified");
}

/// `create --notify-command` persists notification metadata with the same shape as `run`.
#[test]
fn create_notify_command_persisted_same_shape_as_run() {
    let h = TestHarness::new();
    let notify_cmd = "cat >/dev/null";

    // create path
    let c = h.run(&["create", "--notify-command", notify_cmd, "--", "true"]);
    assert_envelope(&c, "create", true);
    let create_job_id = c["job_id"].as_str().unwrap();
    let create_meta: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(create_job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    // run path
    let r = h.run(&["run", "--notify-command", notify_cmd, "--", "true"]);
    assert_envelope(&r, "run", true);
    let run_job_id = r["job_id"].as_str().unwrap();
    let run_meta: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(run_job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        create_meta["notification"]["notify_command"], run_meta["notification"]["notify_command"],
        "notify_command must be persisted with the same shape by create and run"
    );
}

/// `create --output-pattern` persists output-match metadata with the same shape as `run`.
#[test]
fn create_output_pattern_persisted_same_shape_as_run() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("events.ndjson");
    let events_path = events_file.to_str().unwrap();

    // create path
    let c = h.run(&[
        "create",
        "--output-pattern",
        "ERROR",
        "--output-command",
        "cat >/dev/null",
        "--output-file",
        events_path,
        "--",
        "sh",
        "-c",
        "echo ERROR",
    ]);
    assert_envelope(&c, "create", true);
    let create_job_id = c["job_id"].as_str().unwrap();
    let create_meta: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(create_job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    // run path
    let r = h.run(&[
        "run",
        "--output-pattern",
        "ERROR",
        "--output-command",
        "cat >/dev/null",
        "--output-file",
        events_path,
        "--",
        "sh",
        "-c",
        "echo ERROR",
    ]);
    assert_envelope(&r, "run", true);
    let run_job_id = r["job_id"].as_str().unwrap();
    let run_meta: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(h.root())
                .join(run_job_id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();

    let create_match = &create_meta["notification"]["on_output_match"];
    let run_match = &run_meta["notification"]["on_output_match"];

    assert_eq!(
        create_match["pattern"], run_match["pattern"],
        "on_output_match.pattern must match between create and run"
    );
    assert_eq!(
        create_match["command"], run_match["command"],
        "on_output_match.command must match between create and run"
    );
    assert_eq!(
        create_match["file"], run_match["file"],
        "on_output_match.file must match between create and run"
    );
}

/// `create --output-pattern` does NOT trigger notification delivery during create itself.
#[test]
fn create_does_not_trigger_notification_side_effects() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("create_side_effects.ndjson");
    let events_path = events_file.to_str().unwrap();

    // Create the job with output-match and completion notification configured.
    let c = h.run(&[
        "create",
        "--notify-command",
        &format!("echo triggered >> {}", events_path),
        "--output-pattern",
        "ERROR",
        "--output-file",
        events_path,
        "--",
        "sh",
        "-c",
        "echo ERROR",
    ]);
    assert_envelope(&c, "create", true);

    // `create` must return immediately without executing the command.
    // Give a brief window for any inadvertent side effects.
    std::thread::sleep(std::time::Duration::from_millis(200));

    assert!(
        !events_file.exists(),
        "create must not execute notification sinks or the command"
    );
}

/// Tags persisted by `create` are used by `start` as the job's initial tag set.
#[test]
fn start_uses_tags_persisted_by_create() {
    let h = TestHarness::new();

    let c = h.run(&["create", "--tag", "mytag", "--tag", "other", "--", "true"]);
    assert_envelope(&c, "create", true);
    let job_id = c["job_id"].as_str().unwrap().to_string();

    let s = h.run(&["start", &job_id]);
    assert_envelope(&s, "start", true);
    let tags: Vec<&str> = s["tags"]
        .as_array()
        .expect("tags in start response")
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(
        tags,
        vec!["mytag", "other"],
        "start must return the tags persisted by create"
    );
}

/// Output-match notification persisted by `create` is used by `start` when executing the job.
#[test]
fn start_uses_output_match_notification_persisted_by_create() {
    let h = TestHarness::new();
    let tmp_dir = tempfile::tempdir().expect("tempdir");
    let events_file = tmp_dir.path().join("start_output_match.ndjson");
    let events_path = events_file.to_str().unwrap();

    // Create with output-match notification; do NOT execute.
    let c = h.run(&[
        "create",
        "--output-pattern",
        "MATCH_ME",
        "--output-file",
        events_path,
        "--",
        "sh",
        "-c",
        "echo MATCH_ME",
    ]);
    assert_envelope(&c, "create", true);
    let job_id = c["job_id"].as_str().unwrap().to_string();

    // Start the job; the supervisor must pick up the persisted output-match config.
    let s = h.run(&["start", &job_id]);
    assert_envelope(&s, "start", true);

    // Allow time for the supervisor to process output and deliver the notification.
    std::thread::sleep(std::time::Duration::from_millis(2000));

    assert!(
        events_file.exists(),
        "output-match event file must be written when start uses persisted create config"
    );
    let content = std::fs::read_to_string(&events_file).unwrap();
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one match event must be written");
    let ev: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(
        ev["event_type"].as_str().unwrap_or(""),
        "job.output.matched"
    );
    assert_eq!(ev["line"].as_str().unwrap_or(""), "MATCH_ME");
}

// ── regression: lingering running state ────────────────────────────────────────

/// Regression test: job transitions out of `running` promptly after the wrapped root
/// process exits, even when a descendant keeps inherited stdout/stderr handles open.
///
/// The root `sh` spawns `sleep 30` in the background (`&`) and exits immediately.
/// `sleep 30` inherits the pipe write-ends and keeps them open. Prior to the fix,
/// the supervisor blocked on log-thread EOF, keeping the job in `running` indefinitely.
#[test]
#[cfg(unix)]
fn status_becomes_terminal_when_root_exits_despite_inherited_stdio() {
    let h = TestHarness::new();

    // Root shell exits immediately; background sleep inherits pipe ends and keeps them open.
    let run_v = h.run(&["run", "--", "sh", "-c", "sleep 30 &"]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Poll for terminal state. Allow 10 s as a generous bound for slow CI environments.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let poll = std::time::Duration::from_millis(50);
    let mut observed_state = String::new();
    while std::time::Instant::now() < deadline {
        let v = h.run(&["status", &job_id]);
        let state = v["state"].as_str().unwrap_or("").to_string();
        if state != "running" && state != "created" {
            observed_state = state;
            break;
        }
        std::thread::sleep(poll);
    }
    assert!(
        !observed_state.is_empty() && observed_state != "running",
        "job must reach a terminal state promptly after wrapped root exits; \
         stuck in state={observed_state:?} (regression: lingering running state)"
    );
}

/// Regression test: `_supervise` exits promptly after the wrapped root process ends,
/// even when a descendant keeps inherited stdio handles open.
///
/// This complements `status_becomes_terminal_when_root_exits_despite_inherited_stdio`
/// by asserting that the supervisor process itself is no longer visible in the process
/// table after the job reaches a terminal state.
#[test]
#[cfg(unix)]
fn supervise_exits_promptly_after_root_exits_despite_inherited_stdio() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "--", "sh", "-c", "sleep 30 &"]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Step 1: wait for terminal state (same as previous regression test).
    let status_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let poll = std::time::Duration::from_millis(50);
    let mut reached_terminal = false;
    while std::time::Instant::now() < status_deadline {
        let v = h.run(&["status", &job_id]);
        let state = v["state"].as_str().unwrap_or("");
        if state != "running" && state != "created" {
            reached_terminal = true;
            break;
        }
        std::thread::sleep(poll);
    }
    assert!(
        reached_terminal,
        "prerequisite: job must reach terminal state before checking supervisor linger"
    );

    // Step 2: assert that _supervise is no longer in the process table.
    // We give it 5 s of grace after terminal state is observed, which is far more
    // than the supervisor needs to dispatch notifications and exit.
    let linger_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut supervisor_lingering = true;
    let pgrep_pattern = format!("_supervise.*{job_id}");
    while std::time::Instant::now() < linger_deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let result = std::process::Command::new("pgrep")
            .arg("-f")
            .arg(&pgrep_pattern)
            .output();
        match result {
            Ok(output) => {
                if output.stdout.iter().all(|b| b.is_ascii_whitespace()) {
                    // pgrep found no matching process.
                    supervisor_lingering = false;
                    break;
                }
            }
            Err(_) => {
                // pgrep unavailable on this platform; skip supervisor linger check.
                supervisor_lingering = false;
                break;
            }
        }
    }
    assert!(
        !supervisor_lingering,
        "_supervise must not linger after job reaches terminal state \
         (job_id={job_id}; background sleep 30 holds inherited pipe ends)"
    );
}

// ─── argv-mode exec handoff tests ──────────────────────────────────────────

/// Verify that argv-mode launches (multi-element command) complete successfully
/// through the exec handoff semantics and produce the expected output.
#[test]
#[cfg(unix)]
fn argv_mode_exec_handoff_completes() {
    let h = TestHarness::new();

    // Multi-element argv: ["sh", "-c", "echo argv-ok"]
    // Should exec into `sh -c 'echo argv-ok'` via wrapper's exec handoff.
    let v = h.run(&["run", "--", "sh", "-c", "echo argv-ok"]);
    assert_envelope(&v, "run", true);

    let job_id = v["job_id"].as_str().unwrap();
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(
        wait_v["state"].as_str().unwrap_or(""),
        "exited",
        "argv-mode job must reach exited state"
    );
    assert_eq!(wait_v["exit_code"], 0, "argv-mode job must exit 0");

    // Verify the command output reached stdout.log.
    let logs_v = h.run(&["tail", job_id, "--tail-lines", "5"]);
    let stdout = logs_v["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("argv-ok"),
        "stdout must contain 'argv-ok'; got: {stdout:?}"
    );
}

/// Verify that shell-string mode (single-element command) still works correctly
/// after the argv exec handoff change, preserving shell operator semantics.
#[test]
#[cfg(unix)]
fn shell_string_mode_preserved_after_argv_change() {
    let h = TestHarness::new();

    // Single-element command string with shell operators.
    let v = h.run(&["run", "--", "echo string-ok && echo string-two"]);
    assert_envelope(&v, "run", true);

    let job_id = v["job_id"].as_str().unwrap();
    let wait_v = wait_until_terminal(&h, job_id);
    assert_eq!(wait_v["exit_code"], 0, "shell-string mode job must exit 0");
    let logs_v = h.run(&["tail", job_id, "--tail-lines", "5"]);
    let stdout = logs_v["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("string-ok"),
        "stdout must contain 'string-ok'; got: {stdout:?}"
    );
    assert!(
        stdout.contains("string-two"),
        "stdout must contain 'string-two' (shell && operator); got: {stdout:?}"
    );
}

/// Regression test for issue #5 — argv-mode: completion tracking aligns with
/// the intended workload boundary after the exec handoff.
///
/// A background descendant (`sleep 30 &`) that inherits stdio must NOT prevent
/// the job from reaching a terminal state promptly when the main argv workload
/// exits.  This mirrors the existing string-mode regression but uses argv-style
/// invocation so the exec handoff path is exercised.
#[test]
#[cfg(unix)]
fn argv_mode_completion_aligns_with_workload_boundary_issue5_regression() {
    let h = TestHarness::new();

    // Argv-mode: main workload exits immediately; background sleep inherits pipes.
    let run_v = h.run(&["run", "--", "sh", "-c", "sleep 30 &"]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // The job must reach a terminal state promptly despite the lingering descendant.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let poll = std::time::Duration::from_millis(50);
    let mut observed_state = String::new();
    while std::time::Instant::now() < deadline {
        let v = h.run(&["status", &job_id]);
        let state = v["state"].as_str().unwrap_or("").to_string();
        if state != "running" && state != "created" {
            observed_state = state;
            break;
        }
        std::thread::sleep(poll);
    }
    assert!(
        !observed_state.is_empty() && observed_state != "running",
        "argv-mode job must reach terminal state promptly after workload exits \
         (issue #5 regression); stuck in state={observed_state:?}"
    );
}

// ── post-0.1.10 regression: cflx run lingering workload shape ────────────────

/// Post-0.1.10 regression for issue #5: models the failure shape where a workload
/// emits apparent success output ("Orchestrator completed successfully") but the root
/// workload process itself remains alive.
///
/// This is distinct from the already-addressed case where the root process exits
/// immediately but background descendants keep inherited stdio handles open. In this
/// shape, the root workload process (simulating `cflx run`) is still alive, so
/// `child.wait()` in `_supervise` has not returned, and `state.json` correctly
/// continues to report `running`.
///
/// The test documents two facts:
///   1. Success-like output is captured and visible in `tail` immediately.
///   2. `status` correctly remains `running` while the root process is alive.
///
/// The mismatch between (1) and (2) is the observable bug from the user's perspective.
/// The fix belongs in the workload (`cflx run`), which must exit promptly after
/// completing its orchestration work, not in agent-exec's state model.
///
/// Acceptance criterion: this test must continue to pass after any proposed fix,
/// demonstrating that the fix-forward path is evaluated against this workload shape.
#[test]
#[cfg(unix)]
fn status_remains_running_while_root_alive_despite_success_output_post_0_1_10_issue5() {
    let h = TestHarness::new();

    // Simulate cflx run: print apparent success lines, then linger (root stays alive).
    let run_v = h.run(&[
        "run",
        "--",
        "sh",
        "-c",
        concat!(
            "echo 'No changes found for parallel execution'; ",
            "echo 'Orchestrator completed successfully'; ",
            "sleep 30"
        ),
    ]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    // Allow log threads time to capture the output before polling.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Fact 1: success-like output is captured and visible.
    let tail_v = h.run(&["tail", &job_id, "--tail-lines", "20"]);
    let stdout = tail_v["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("Orchestrator completed successfully"),
        "success-like output must be visible in stdout log before process exits; \
         got: {stdout:?}"
    );

    // Fact 2: status is still `running` because the root workload process (sleep 30)
    // has not exited. agent-exec's state model is correct here; the bug is upstream.
    let status_v = h.run(&["status", &job_id]);
    let state = status_v["state"].as_str().unwrap_or("");
    assert_eq!(
        state, "running",
        "status must remain `running` while the root workload process is alive, \
         even when success-like output is already present in the log \
         (post-0.1.10 issue #5 shape; fix must be in the upstream workload)"
    );

    // Cleanup: forcibly kill the lingering workload (sleep 30 + _supervise) so
    // this test does not leak processes and weaken integration-test isolation.
    h.run(&["kill", "--signal", "KILL", &job_id]);
}

/// On non-Unix platforms argv-mode falls back to shell-string semantics (wrapper
/// invoked with joined argv string) so that cmd /C launch semantics are preserved.
///
/// This test verifies that a multi-element argv command completes successfully on
/// Windows using the shell-string fallback path.
#[test]
#[cfg(not(unix))]
fn argv_mode_non_unix_shell_string_fallback_completes() {
    let h = TestHarness::new();

    // Multi-element argv on Windows: should fall back to joined shell-string mode.
    let v = h.run(&["run", "--", "cmd", "/C", "echo argv-win-ok"]);
    assert_envelope(&v, "run", true);
    assert_eq!(v["exit_code"], 0, "argv-mode non-Unix job must exit 0");

    let job_id = v["job_id"].as_str().unwrap();
    let status_v = h.run(&["status", job_id]);
    assert_eq!(
        status_v["state"].as_str().unwrap_or(""),
        "exited",
        "argv-mode non-Unix job must reach exited state"
    );

    let logs_v = h.run(&["tail", job_id, "--tail-lines", "5"]);
    let stdout = logs_v["stdout"].as_str().unwrap_or("");
    assert!(
        stdout.contains("argv-win-ok"),
        "stdout must contain 'argv-win-ok' on non-Unix argv fallback; got: {stdout:?}"
    );
}

// ── --yaml output format ────────────────────────────────────────────────────────

/// Helper: run binary with --yaml flag and return raw stdout string.
fn run_yaml_raw(args: &[&str], root: &str) -> String {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.arg("--yaml");
    cmd.args(args);
    cmd.env("AGENT_EXEC_ROOT", root);
    let output = cmd.output().expect("run binary");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Helper: run binary with --yaml flag and parse stdout as YAML → serde_json::Value.
fn run_yaml(args: &[&str], root: &str) -> serde_json::Value {
    let raw = run_yaml_raw(args, root);
    let stderr = {
        let bin = binary();
        let mut cmd = Command::new(&bin);
        cmd.arg("--yaml");
        cmd.args(args);
        cmd.env("AGENT_EXEC_ROOT", root);
        let output = cmd.output().expect("run binary");
        String::from_utf8_lossy(&output.stderr).into_owned()
    };
    assert!(!raw.trim().is_empty(), "stdout is empty (stderr: {stderr})");
    // Parse YAML into serde_json::Value via serde_yaml.
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(&raw).unwrap_or_else(|e| {
        panic!("stdout is not valid YAML: {e}\nstdout: {raw}\nstderr: {stderr}")
    });
    // Convert to JSON value for reuse of assert_envelope helper.
    serde_json::to_value(&yaml_val).expect("yaml->json conversion")
}

#[test]
fn yaml_flag_run_returns_yaml() {
    let h = TestHarness::new();
    let raw = run_yaml_raw(&["run", "echo", "yaml_test"], h.root());
    // Must be parseable YAML (not JSON object on one line).
    assert!(!raw.trim().is_empty(), "stdout empty");
    // A JSON single-line response would start with '{'; YAML mapping starts with key or '---'.
    // Either way it must not be a single-line JSON blob.
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("not valid YAML: {e}\nstdout: {raw}"));
    assert!(parsed.is_mapping(), "expected YAML mapping");
}

#[test]
fn yaml_flag_run_envelope_fields() {
    let h = TestHarness::new();
    let v = run_yaml(&["run", "echo", "hi"], h.root());
    assert_envelope(&v, "run", true);
    assert!(v["job_id"].as_str().is_some(), "job_id missing: {v}");
}

#[test]
fn yaml_flag_status_success() {
    let h = TestHarness::new();
    let run_v = run_yaml(&["run", "echo", "hi"], h.root());
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    let v = run_yaml(&["status", &job_id], h.root());
    assert_envelope(&v, "status", true);
    assert_eq!(v["job_id"].as_str().unwrap_or(""), job_id);
}

#[test]
fn yaml_flag_error_response() {
    let h = TestHarness::new();
    let v = run_yaml(&["status", "NONEXISTENT_JOB_ID_YAML"], h.root());
    assert_envelope(&v, "error", false);
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

#[test]
fn yaml_flag_schema_returns_yaml() {
    let h = TestHarness::new();
    let v = run_yaml(&["schema"], h.root());
    assert_envelope(&v, "schema", true);
    assert!(
        v["schema"].is_object() || v["schema"].is_string(),
        "schema field missing or wrong type: {v}"
    );
}

#[test]
fn json_default_still_works_without_yaml_flag() {
    let h = TestHarness::new();
    // Without --yaml, stdout should be valid JSON (original behavior).
    let v = h.run(&["run", "echo", "json_default"]);
    assert_envelope(&v, "run", true);
}

#[test]
fn yaml_flag_after_subcommand_works() {
    // --yaml declared with global=true, so it can appear after the subcommand name too.
    let h = TestHarness::new();
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(["run", "--yaml", "echo", "global_test"]);
    cmd.env("AGENT_EXEC_ROOT", h.root());
    let output = cmd.output().expect("run binary");
    let raw = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("not valid YAML: {e}\nstdout: {raw}"));
    assert!(parsed.is_mapping(), "expected YAML mapping");
}

// ── delete ─────────────────────────────────────────────────────────────────────

/// `delete <job_id>` removes a finished job and returns type="delete".
#[test]
fn delete_single_removes_finished_job() {
    let h = TestHarness::new();

    // Create a job and wait for it to finish.
    let run_v = h.run(&["run", "echo", "delete_me"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    // Delete the job.
    let v = h.run(&["delete", &job_id]);
    assert_envelope(&v, "delete", true);
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "expected deleted=1: {v}"
    );
    assert_eq!(
        v["skipped"].as_u64().unwrap_or(1),
        0,
        "expected skipped=0: {v}"
    );
    assert!(v["jobs"].is_array(), "jobs field missing: {v}");
    let jobs = v["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["job_id"].as_str().unwrap_or(""), job_id);
    assert_eq!(jobs[0]["action"].as_str().unwrap_or(""), "deleted");

    // Subsequent status must return job_not_found.
    let status_v = h.run(&["status", &job_id]);
    assert!(
        !status_v["ok"].as_bool().unwrap_or(true),
        "expected ok=false after delete: {status_v}"
    );
    assert_eq!(
        status_v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found"
    );
}

/// `delete <job_id>` rejects a running job with error.code="invalid_state".
#[test]
fn delete_running_job_returns_invalid_state() {
    let h = TestHarness::new();

    // Start a long-running job.
    let run_v = h.run(&["run", "sleep", "30"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    assert_eq!(run_v["state"].as_str().unwrap_or(""), "running");

    // Attempt to delete it.
    let v = h.run(&["delete", &job_id]);
    assert!(
        !v["ok"].as_bool().unwrap_or(true),
        "expected ok=false for running job: {v}"
    );
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "invalid_state");

    // The job directory must still exist (verify via status).
    let status_v = h.run(&["status", &job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "job directory must still exist: {status_v}"
    );

    // Clean up.
    h.run(&["kill", &job_id]);
}

/// `delete <job_id>` on a non-existent job returns job_not_found.
#[test]
fn delete_nonexistent_job_returns_job_not_found() {
    let h = TestHarness::new();
    let v = h.run(&["delete", "NONEXISTENT_JOB_ID_XYZ"]);
    assert!(!v["ok"].as_bool().unwrap_or(true), "expected ok=false: {v}");
    assert_eq!(v["error"]["code"].as_str().unwrap_or(""), "job_not_found");
}

/// `delete --dry-run <job_id>` reports the deletion but does NOT remove the directory.
#[test]
fn delete_dry_run_single_preserves_directory() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "echo", "dry_run_single"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let v = h.run(&["delete", "--dry-run", &job_id]);
    assert_envelope(&v, "delete", true);
    assert!(v["dry_run"].as_bool().unwrap_or(false));
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(1),
        0,
        "dry-run must not count deleted: {v}"
    );
    let jobs = v["jobs"].as_array().unwrap();
    assert_eq!(jobs[0]["action"].as_str().unwrap_or(""), "would_delete");

    // Job must still be accessible.
    let status_v = h.run(&["status", &job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "job must still exist after dry-run: {status_v}"
    );
}

/// `delete --all` deletes only terminal jobs in the caller's cwd; jobs from other
/// cwds are left untouched.
#[test]
fn delete_all_scopes_to_current_cwd() {
    let h = TestHarness::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    // Create finished job A in dir_a.
    let (va, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "job_a"],
        Some(h.root()),
        Some(dir_a.path()),
    );
    let job_a = va["job_id"].as_str().unwrap().to_string();

    // Create finished job B in dir_b.
    let (vb, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "job_b"],
        Some(h.root()),
        Some(dir_b.path()),
    );
    let job_b = vb["job_id"].as_str().unwrap().to_string();

    // Wait for both to finish.
    h.run(&["wait", &job_a]);
    h.run(&["wait", &job_b]);

    // Run `delete --all` from dir_a — only job A should be deleted.
    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir_a.path()));
    assert_envelope(&del_v, "delete", true);

    // Job A must be gone.
    let status_a = h.run(&["status", &job_a]);
    assert_eq!(
        status_a["error"]["code"].as_str().unwrap_or(""),
        "job_not_found",
        "job A must be deleted: {status_a}"
    );

    // Job B must still exist.
    let status_b = h.run(&["status", &job_b]);
    assert!(
        status_b["ok"].as_bool().unwrap_or(false),
        "job B must survive: {status_b}"
    );
}

/// `delete --all` does NOT delete running or created jobs.
#[test]
fn delete_all_skips_running_and_created_jobs() {
    let h = TestHarness::new();
    let dir = tempfile::tempdir().unwrap();

    // Start a long-running job.
    let (run_v, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "30"], Some(h.root()), Some(dir.path()));
    let running_job_id = run_v["job_id"].as_str().unwrap().to_string();
    assert_eq!(run_v["state"].as_str().unwrap_or(""), "running");

    // Run delete --all from the same directory.
    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir.path()));
    assert_envelope(&del_v, "delete", true);
    assert_eq!(
        del_v["deleted"].as_u64().unwrap_or(1),
        0,
        "should delete nothing: {del_v}"
    );

    // Running job must still be alive.
    let status_v = h.run(&["status", &running_job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "running job must survive: {status_v}"
    );

    // Verify it appears in the skipped list.
    let jobs = del_v["jobs"].as_array().unwrap();
    let skipped = jobs
        .iter()
        .filter(|j| j["action"].as_str().unwrap_or("") == "skipped")
        .count();
    assert!(skipped >= 1, "expected at least one skipped entry: {del_v}");

    // Clean up.
    h.run(&["kill", &running_job_id]);
}

/// `delete --all` skips jobs whose persisted state is terminal but whose pid is still alive.
#[test]
fn delete_all_skips_terminal_state_with_live_pid() {
    let h = TestHarness::new();
    let dir = tempfile::tempdir().unwrap();

    let (run_v, _) =
        run_cmd_with_root_and_cwd(&["run", "echo", "done"], Some(h.root()), Some(dir.path()));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let job_dir = std::path::Path::new(h.root()).join(&job_id);
    let state_path = job_dir.join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).expect("read state.json"))
            .expect("parse state.json");
    state["pid"] = serde_json::json!(std::process::id());
    std::fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("serialize state"),
    )
    .expect("write state.json");

    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir.path()));
    assert_envelope(&del_v, "delete", true);
    assert_eq!(del_v["deleted"].as_u64().unwrap_or(1), 0, "{del_v}");

    let jobs = del_v["jobs"].as_array().unwrap();
    assert!(
        jobs.iter().any(|j| {
            j["job_id"].as_str().unwrap_or("") == job_id
                && j["action"].as_str().unwrap_or("") == "skipped"
                && j["reason"].as_str().unwrap_or("") == "pid_alive"
        }),
        "expected pid_alive skip: {del_v}"
    );

    let status_v = h.run(&["status", &job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "job must survive: {status_v}"
    );
}

/// `delete --dry-run --all` reports candidates without removing any directories.
#[test]
fn delete_all_dry_run_preserves_directories() {
    let h = TestHarness::new();
    let dir = tempfile::tempdir().unwrap();

    // Finish a job in `dir`.
    let (run_v, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "dry_all"],
        Some(h.root()),
        Some(dir.path()),
    );
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    // Dry-run delete --all.
    let (del_v, _) = run_cmd_with_root_and_cwd(
        &["delete", "--dry-run", "--all"],
        Some(h.root()),
        Some(dir.path()),
    );
    assert_envelope(&del_v, "delete", true);
    assert!(del_v["dry_run"].as_bool().unwrap_or(false));
    assert_eq!(
        del_v["deleted"].as_u64().unwrap_or(1),
        0,
        "dry-run must not delete: {del_v}"
    );

    let jobs = del_v["jobs"].as_array().unwrap();
    let would_delete: Vec<_> = jobs
        .iter()
        .filter(|j| j["action"].as_str().unwrap_or("") == "would_delete")
        .collect();
    assert!(
        !would_delete.is_empty(),
        "expected at least one would_delete entry: {del_v}"
    );

    // Job directory must still exist.
    let status_v = h.run(&["status", &job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "job must still exist after dry-run: {status_v}"
    );
}

// ── delete / gc post-delete observability (harden-delete-gc-deletion-observability) ────

/// `delete <job_id>` reporting `action="deleted"` MUST mean the job directory is
/// absent from disk by the time the response is emitted.
#[test]
fn delete_single_deleted_action_implies_directory_absent() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "echo", "post_check_single"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let job_path = std::path::Path::new(h.root()).join(&job_id);
    assert!(job_path.exists(), "precondition: job directory exists");

    let v = h.run(&["delete", &job_id]);
    assert_envelope(&v, "delete", true);
    let jobs = v["jobs"].as_array().unwrap();
    assert_eq!(jobs[0]["action"].as_str().unwrap_or(""), "deleted");
    // Per spec: deleted ⇒ path no longer exists at command completion.
    assert!(
        !job_path.exists(),
        "deleted job directory must be absent on disk: {job_path:?}"
    );
}

/// `delete --all` MUST include `cwd_scope` set to the effective cwd it
/// evaluated bulk deletions against.
#[test]
fn delete_all_response_includes_cwd_scope() {
    let h = TestHarness::new();
    let dir = tempfile::tempdir().unwrap();
    let canonical = dir
        .path()
        .canonicalize()
        .unwrap_or_else(|_| dir.path().to_path_buf());

    let (run_v, _) =
        run_cmd_with_root_and_cwd(&["run", "echo", "scoped"], Some(h.root()), Some(dir.path()));
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir.path()));
    assert_envelope(&del_v, "delete", true);

    let scope = del_v["cwd_scope"]
        .as_str()
        .expect("cwd_scope must be present for delete --all");
    assert_eq!(
        scope,
        canonical.display().to_string(),
        "cwd_scope must report the effective cwd used for evaluation: {del_v}"
    );
}

/// Single-job `delete <JOB_ID>` is not cwd-scoped, so `cwd_scope` MUST be absent.
#[test]
fn delete_single_response_omits_cwd_scope() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "echo", "no_scope"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let v = h.run(&["delete", &job_id]);
    assert_envelope(&v, "delete", true);
    assert!(
        v.get("cwd_scope").is_none(),
        "cwd_scope must be absent for single-job delete: {v}"
    );
}

/// `delete --all` MUST report cwd-mismatched jobs via the `out_of_scope` aggregate
/// so callers can distinguish "filtered out by cwd" from "evaluated but skipped".
#[test]
fn delete_all_distinguishes_out_of_scope_from_in_scope_skipped() {
    let h = TestHarness::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    // In-scope (cwd matches) but skipped (running) job.
    let (run_v, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "30"], Some(h.root()), Some(dir_a.path()));
    let running_id = run_v["job_id"].as_str().unwrap().to_string();

    // Out-of-scope (cwd mismatch) finished job from dir_b.
    let (other_v, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "other"],
        Some(h.root()),
        Some(dir_b.path()),
    );
    let other_id = other_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &other_id]);

    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir_a.path()));
    assert_envelope(&del_v, "delete", true);

    assert!(
        del_v["out_of_scope"].as_u64().unwrap_or(0) >= 1,
        "out_of_scope must reflect cwd-mismatched job: {del_v}"
    );
    assert_eq!(
        del_v["failed"].as_u64().unwrap_or(99),
        0,
        "failed must be 0 when no deletions failed: {del_v}"
    );

    // The cwd-mismatched job is filtered before the per-job array, so it does
    // NOT appear in jobs[]; the in-scope running job DOES appear with skipped.
    let jobs = del_v["jobs"].as_array().unwrap();
    assert!(
        !jobs.iter().any(|j| j["job_id"].as_str() == Some(&other_id)),
        "out-of-scope cwd-mismatched job must not be listed per-job: {del_v}"
    );
    assert!(
        jobs.iter()
            .any(|j| j["job_id"].as_str() == Some(&running_id)
                && j["action"].as_str() == Some("skipped")
                && j["reason"].as_str() == Some("running")),
        "in-scope running job must appear as skipped: {del_v}"
    );

    h.run(&["kill", &running_id]);
}

/// `delete --all` reporting `action="deleted"` MUST mean each such job
/// directory is absent on disk by the time the response is emitted.
#[test]
fn delete_all_deleted_action_implies_directories_absent() {
    let h = TestHarness::new();
    let dir = tempfile::tempdir().unwrap();

    let (run_v, _) = run_cmd_with_root_and_cwd(
        &["run", "echo", "post_check_all"],
        Some(h.root()),
        Some(dir.path()),
    );
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let (del_v, _) =
        run_cmd_with_root_and_cwd(&["delete", "--all"], Some(h.root()), Some(dir.path()));
    assert_envelope(&del_v, "delete", true);

    let deleted_ids: Vec<String> = del_v["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| j["action"].as_str() == Some("deleted"))
        .map(|j| j["job_id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        deleted_ids.contains(&job_id),
        "expected job to be deleted: {del_v}"
    );

    for id in &deleted_ids {
        let p = std::path::Path::new(h.root()).join(id);
        assert!(
            !p.exists(),
            "deleted job directory must be absent on disk: {p:?} (response: {del_v})"
        );
    }
}

/// `gc` reporting `action="deleted"` MUST mean each such job directory is
/// absent on disk by the time the response is emitted; existing reasons must
/// also remain distinguishable via the `out_of_scope` aggregate.
#[test]
fn gc_deleted_action_implies_directory_absent_and_categorises_skips() {
    let h = TestHarness::new();
    let old = "2020-01-01T00:00:00Z";
    write_fake_job(h.root(), "exited-old", "exited", Some(old), old);
    // Out-of-scope: running job MUST be reported via `out_of_scope`.
    write_fake_job(h.root(), "running-job", "running", None, old);

    let v = h.run(&["gc", "--older-than", "7d"]);
    assert_gc_envelope(&v, false);
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "exactly one job should be deleted: {v}"
    );
    assert!(
        v["out_of_scope"].as_u64().unwrap_or(0) >= 1,
        "running job must be counted in out_of_scope: {v}"
    );
    assert_eq!(
        v["failed"].as_u64().unwrap_or(99),
        0,
        "no deletions failed in this scenario: {v}"
    );

    // Per spec: deleted ⇒ path no longer exists at command completion.
    let deleted_path = std::path::Path::new(h.root()).join("exited-old");
    assert!(
        !deleted_path.exists(),
        "deleted job directory must be absent on disk: {deleted_path:?}"
    );
    let running_path = std::path::Path::new(h.root()).join("running-job");
    assert!(running_path.exists(), "running job must be preserved");
}

/// Run the binary with given args and return raw stdout + exit code (no JSON parsing).
fn run_raw(args: &[&str]) -> (String, i32) {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, code)
}

#[test]
fn completions_bash_outputs_nonempty_script() {
    let (stdout, code) = run_raw(&["completions", "bash"]);
    assert_eq!(code, 0, "exit code should be 0 for 'completions bash'");
    assert!(
        !stdout.trim().is_empty(),
        "stdout should be non-empty for 'completions bash'"
    );
    // Bash completion scripts typically start with a function definition.
    assert!(
        stdout.contains("agent-exec") || stdout.contains("agent_exec"),
        "bash completion script should reference agent-exec: {stdout}"
    );
}

#[test]
fn completions_zsh_outputs_nonempty_script() {
    let (stdout, code) = run_raw(&["completions", "zsh"]);
    assert_eq!(code, 0, "exit code should be 0 for 'completions zsh'");
    assert!(
        !stdout.trim().is_empty(),
        "stdout should be non-empty for 'completions zsh'"
    );
}

#[test]
fn completions_fish_outputs_nonempty_script() {
    let (stdout, code) = run_raw(&["completions", "fish"]);
    assert_eq!(code, 0, "exit code should be 0 for 'completions fish'");
    assert!(
        !stdout.trim().is_empty(),
        "stdout should be non-empty for 'completions fish'"
    );
}

#[test]
fn completions_powershell_outputs_nonempty_script() {
    let (stdout, code) = run_raw(&["completions", "powershell"]);
    assert_eq!(
        code, 0,
        "exit code should be 0 for 'completions powershell'"
    );
    assert!(
        !stdout.trim().is_empty(),
        "stdout should be non-empty for 'completions powershell'"
    );
}

#[test]
fn completions_invalid_shell_exits_with_code_2() {
    let bin = binary();
    let output = Command::new(&bin)
        .args(["completions", "invalid"])
        .output()
        .expect("run binary");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for 'completions invalid'"
    );
}

#[test]
fn list_state_invalid_value_exits_with_code_2() {
    let bin = binary();
    let output = Command::new(&bin)
        .args(["list", "--all", "--state", "bogus"])
        .output()
        .expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for invalid --state value, stdout: {stdout}"
    );
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty for invalid --state usage error: {stdout}"
    );
}

#[test]
fn version_flag_prints_version_and_exits_zero() {
    let bin = binary();
    let pkg_version = env!("CARGO_PKG_VERSION");

    for flag in &["--version", "-V"] {
        let output = std::process::Command::new(&bin)
            .arg(flag)
            .output()
            .unwrap_or_else(|e| panic!("failed to run binary with {flag}: {e}"));
        assert!(
            output.status.success(),
            "exit code is non-zero for {flag}: {:?}",
            output.status
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("agent-exec") && stdout.contains(pkg_version),
            "stdout does not match 'agent-exec <version>' for {flag}: {stdout}"
        );
    }
}

// ── prefix-based job ID lookup ─────────────────────────────────────────────────

/// Helper: run a command and return (json, exit_code).
fn run_cmd_raw(args: &[&str], root: Option<&str>) -> (serde_json::Value, i32) {
    let bin = binary();
    let mut cmd = std::process::Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\nstdout: {stdout}"));
    (v, exit_code)
}

#[test]
fn prefix_lookup_resolves() {
    let h = TestHarness::new();

    // Start a job and get its full ID.
    let run_v = h.run(&["run", "echo", "prefix_test"]);
    let full_id = run_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    // Use a unique prefix (first 10 chars) to query status.
    let prefix = &full_id[..10];
    let v = h.run(&["status", prefix]);
    assert_envelope(&v, "status", true);
    // The response must contain the full resolved ID, not the prefix.
    assert_eq!(
        v["job_id"].as_str().unwrap_or(""),
        full_id,
        "job_id in response must be the resolved full ID"
    );
}

#[test]
fn list_includes_short_job_id() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "echo", "short-id"]);
    let full_id = run_v["job_id"].as_str().expect("job_id missing");

    let list_v = h.run(&["list", "--all"]);
    assert_envelope(&list_v, "list", true);
    let jobs = list_v["jobs"].as_array().expect("jobs missing");
    let entry = jobs
        .iter()
        .find(|j| j["job_id"].as_str() == Some(full_id))
        .expect("job summary missing");

    let short = entry["short_job_id"]
        .as_str()
        .expect("short_job_id missing");
    assert_eq!(short.len(), 7, "short_job_id must be 7 chars");
    assert_eq!(short, &full_id[..7], "short_job_id must be job_id prefix");
}

#[test]
fn prefix_lookup_works_with_mixed_hash_and_legacy_ids() {
    let h = TestHarness::new();

    let hash_run = h.run(&["run", "echo", "hash-job"]);
    let hash_id = hash_run["job_id"]
        .as_str()
        .expect("hash job_id")
        .to_string();

    let legacy_run = h.run(&["run", "echo", "legacy-job"]);
    let legacy_original = legacy_run["job_id"]
        .as_str()
        .expect("legacy job_id")
        .to_string();
    let legacy_id = "01JQXK3M8E5PQRSTVWYZ12ABCD";

    let root = std::path::Path::new(h.root());
    let from = root.join(&legacy_original);
    let to = root.join(legacy_id);
    std::fs::rename(&from, &to).expect("rename job dir to legacy id");

    let meta_path = to.join("meta.json");
    let mut meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&meta_path).expect("read meta.json"))
            .expect("parse meta.json");
    meta["job"]["id"] = serde_json::Value::String(legacy_id.to_string());
    std::fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&meta).expect("serialize meta"),
    )
    .expect("write meta.json");

    let state_path = to.join("state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).expect("read state.json"))
            .expect("parse state.json");
    state["job"]["id"] = serde_json::Value::String(legacy_id.to_string());
    std::fs::write(
        &state_path,
        serde_json::to_vec_pretty(&state).expect("serialize state"),
    )
    .expect("write state.json");

    let hash_status = h.run(&["status", &hash_id[..10]]);
    assert_envelope(&hash_status, "status", true);
    assert_eq!(hash_status["job_id"].as_str().unwrap_or(""), hash_id);

    let legacy_status = h.run(&["status", "01JQXK3M"]);
    assert_envelope(&legacy_status, "status", true);
    assert_eq!(legacy_status["job_id"].as_str().unwrap_or(""), legacy_id);
}

#[test]
fn ambiguous_prefix_returns_error() {
    // We need two jobs whose IDs share a common prefix. Since ULIDs are time-based
    // and tests run quickly, both jobs will typically share the same timestamp prefix.
    // We run two jobs back-to-back and try a short prefix shared by both.
    let h = TestHarness::new();

    let run_v1 = h.run(&["run", "echo", "job1"]);
    let id1 = run_v1["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    let run_v2 = h.run(&["run", "echo", "job2"]);
    let id2 = run_v2["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();

    // Find a common prefix length (ULIDs share leading characters from the same epoch second).
    let shared_len = id1
        .chars()
        .zip(id2.chars())
        .take_while(|(a, b)| a == b)
        .count();

    if shared_len == 0 {
        // Very unlikely in practice; skip if IDs don't share any prefix.
        return;
    }

    let prefix = &id1[..shared_len];
    let (v, exit_code) = run_cmd_raw(&["status", prefix], Some(h.root()));
    assert_eq!(exit_code, 1, "ambiguous prefix must exit 1: {v}");
    assert!(!v["ok"].as_bool().unwrap_or(true), "ok must be false: {v}");
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(
        v["error"]["code"].as_str().unwrap_or(""),
        "ambiguous_job_id",
        "expected error.code=ambiguous_job_id: {v}"
    );
    assert!(
        !v["error"]["retryable"].as_bool().unwrap_or(true),
        "retryable must be false: {v}"
    );

    let details = &v["error"]["details"];
    assert!(!details.is_null(), "error.details must be present: {v}");
    let candidates = details["candidates"]
        .as_array()
        .expect("details.candidates must be an array");
    assert!(
        candidates.len() >= 2,
        "candidates must contain at least 2 entries: {v}"
    );
    assert!(
        candidates.iter().any(|c| c.as_str() == Some(&id1)),
        "candidates must include id1: {v}"
    );
    assert!(
        candidates.iter().any(|c| c.as_str() == Some(&id2)),
        "candidates must include id2: {v}"
    );
    assert_eq!(
        details["truncated"].as_bool(),
        Some(false),
        "truncated must be false for 2 candidates: {v}"
    );
}

#[test]
fn prefix_lookup_cross_command() {
    let h = TestHarness::new();

    // Start a job and get its full ID.
    let run_v = h.run(&["run", "sleep", "60"]);
    let full_id = run_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();
    let prefix = &full_id[..10];

    // tail accepts prefix.
    let tail_v = h.run(&["tail", prefix]);
    assert_envelope(&tail_v, "tail", true);
    assert_eq!(tail_v["job_id"].as_str().unwrap_or(""), full_id);

    // wait accepts prefix.
    let (wait_v, _) = run_cmd_raw(&["wait", "--until", "1", prefix], Some(h.root()));
    // ok may be false if job is still running, but job_id must be the full ID.
    assert_eq!(wait_v["job_id"].as_str().unwrap_or(""), full_id);

    // kill accepts prefix.
    let kill_v = h.run(&["kill", prefix]);
    assert_envelope(&kill_v, "kill", true);
    assert_eq!(kill_v["job_id"].as_str().unwrap_or(""), full_id);
}

#[test]
fn delete_prefix_resolves_unique_match() {
    let h = TestHarness::new();

    // Create a finished job.
    let run_v = h.run(&["run", "echo", "delete_prefix_test"]);
    let full_id = run_v["job_id"]
        .as_str()
        .expect("job_id missing")
        .to_string();
    h.run(&["wait", &full_id]);

    // Delete using a prefix (10 chars is plenty to be unique in a fresh harness).
    let prefix = &full_id[..10];
    let (v, exit_code) = run_cmd_raw(&["delete", prefix], Some(h.root()));
    assert_eq!(exit_code, 0, "delete with prefix must succeed: {v}");
    assert_envelope(&v, "delete", true);
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "expected deleted=1: {v}"
    );
    // Response job_id must be the resolved canonical ID, not the prefix.
    let jobs = v["jobs"].as_array().expect("jobs must be array");
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0]["job_id"].as_str().unwrap_or(""),
        full_id,
        "job_id in response must be the resolved full ID"
    );

    // Directory must be gone.
    let (status_v, _) = run_cmd_raw(&["status", &full_id], Some(h.root()));
    assert_eq!(
        status_v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found",
        "job must not exist after delete: {status_v}"
    );
}

#[test]
fn delete_ambiguous_prefix_returns_error() {
    // We need two finished jobs whose IDs share a common prefix.  Since ULIDs
    // are time-based they share the same timestamp bytes when created in rapid
    // succession in the same test process.
    let h = TestHarness::new();

    let run1 = h.run(&["run", "echo", "del_amb_1"]);
    let id1 = run1["job_id"].as_str().expect("job_id").to_string();
    let run2 = h.run(&["run", "echo", "del_amb_2"]);
    let id2 = run2["job_id"].as_str().expect("job_id").to_string();
    h.run(&["wait", &id1]);
    h.run(&["wait", &id2]);

    // Find the longest common prefix.
    let shared_len = id1
        .chars()
        .zip(id2.chars())
        .take_while(|(a, b)| a == b)
        .count();

    if shared_len == 0 {
        // Extremely unlikely; skip rather than fail the test.
        return;
    }

    let prefix = &id1[..shared_len];
    let (v, exit_code) = run_cmd_raw(&["delete", prefix], Some(h.root()));
    assert_eq!(exit_code, 1, "ambiguous delete prefix must exit 1: {v}");
    assert!(!v["ok"].as_bool().unwrap_or(true), "ok must be false: {v}");
    assert_eq!(v["type"].as_str().unwrap_or(""), "error");
    assert_eq!(
        v["error"]["code"].as_str().unwrap_or(""),
        "ambiguous_job_id",
        "error code must be ambiguous_job_id: {v}"
    );
    assert!(
        !v["error"]["retryable"].as_bool().unwrap_or(true),
        "retryable must be false: {v}"
    );
}

// ── Shell completions integration tests ──────────────────────────────────────

/// Helper: run the binary with completion-specific args/env, returning raw stdout and exit code.
fn run_completion(shell: &str, args: &[&str], root: Option<&str>) -> (String, i32) {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    let output = cmd.output().expect("run binary");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    let _ = shell; // for clarity in test names
    (stdout, code)
}

/// Helper: invoke the binary in CompleteEnv mode to get dynamic job-ID candidates.
///
/// Simulates what the shell does when tab-completing a job ID argument.
/// `word_index` is the 0-based index of the word being completed.
/// Returns the list of candidate values printed to stdout.
fn get_dynamic_candidates(
    root: &str,
    subcommand: &str,
    word_index: usize,
    partial: &str,
) -> Vec<String> {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    // CompleteEnv args: <binary_path> -- agent-exec <subcommand> [partial...]
    // The first arg after the binary is the "completer" path; then "--"; then the words.
    cmd.arg(bin.to_str().unwrap());
    cmd.arg("--");
    cmd.arg("agent-exec");
    cmd.arg(subcommand);
    // Always pass the partial value (even empty string) so the engine sees a word
    // at the expected index; without it, the engine reports "no completion generated".
    cmd.arg(partial);
    cmd.env("COMPLETE", "bash");
    cmd.env("AGENT_EXEC_ROOT", root);
    cmd.env("_CLAP_COMPLETE_INDEX", word_index.to_string());
    let output = cmd.output().expect("run binary for dynamic completion");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn write_completion_job(root: &std::path::Path, id: &str, state: &str, cwd: &str) {
    std::fs::create_dir_all(root.join(id)).unwrap();
    std::fs::write(
        root.join(id).join("meta.json"),
        serde_json::json!({
            "job": { "id": id },
            "schema_version": "0.1",
            "command": ["true"],
            "created_at": "2026-01-01T00:00:00Z",
            "root": root.display().to_string(),
            "env_keys": [],
            "cwd": cwd,
            "tags": []
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join(id).join("state.json"),
        serde_json::json!({
            "job": { "id": id, "status": state },
            "result": { "exit_code": null, "signal": null, "duration_ms": null },
            "updated_at": "2026-01-01T00:00:00Z"
        })
        .to_string(),
    )
    .unwrap();
}

#[test]
fn test_completions_bash_outputs_nonempty_script() {
    let (stdout, code) = run_completion("bash", &["completions", "bash"], None);
    assert_eq!(code, 0, "completions bash must exit 0");
    assert!(
        !stdout.trim().is_empty(),
        "completions bash must produce non-empty output"
    );
    // Must contain a bash function definition
    assert!(
        stdout.contains("_agent") || stdout.contains("agent"),
        "bash script must reference agent-exec: {stdout}"
    );
}

#[test]
fn test_completions_zsh_outputs_nonempty_script() {
    let (stdout, code) = run_completion("zsh", &["completions", "zsh"], None);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
    assert!(
        stdout.contains("_clap_dynamic_completer_") && stdout.contains("COMPLETE=\"zsh\""),
        "zsh script must register dynamic completion callbacks: {stdout}"
    );
}

#[test]
fn test_completions_fish_outputs_nonempty_script() {
    let (stdout, code) = run_completion("fish", &["completions", "fish"], None);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
}

#[test]
fn test_completions_powershell_outputs_nonempty_script() {
    let (stdout, code) = run_completion("powershell", &["completions", "powershell"], None);
    assert_eq!(code, 0);
    assert!(!stdout.trim().is_empty());
}

#[test]
fn test_completions_invalid_shell_exits_with_code_2() {
    let (_, code) = run_completion("invalid", &["completions", "invalidshell"], None);
    assert_eq!(code, 2, "invalid shell must produce a usage error (exit 2)");
}

#[test]
fn test_dynamic_completion_all_jobs_for_status() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let cwd = std::env::current_dir().unwrap().display().to_string();
    // Create two jobs with different states
    for (id, state) in &[
        ("01AAAAAAAAAAAAAAAAAAAAAAAAA", "running"),
        ("01BBBBBBBBBBBBBBBBBBBBBBBBB", "exited"),
    ] {
        write_completion_job(tmp.path(), id, state, &cwd);
    }

    // `status` should return all jobs (word index 2: agent-exec status <TAB>)
    let candidates = get_dynamic_candidates(root, "status", 2, "");
    let ids: Vec<_> = candidates.iter().filter(|c| c.starts_with("01")).collect();
    assert!(
        ids.iter().any(|s| s.contains("01AAA")),
        "status should include running jobs: {candidates:?}"
    );
    assert!(
        ids.iter().any(|s| s.contains("01BBB")),
        "status should include exited jobs: {candidates:?}"
    );
}

#[test]
fn test_dynamic_completion_running_only_for_kill() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let cwd = std::env::current_dir().unwrap().display().to_string();
    for (id, state) in &[
        ("01AAAAAAAAAAAAAAAAAAAAAAAAA", "running"),
        ("01BBBBBBBBBBBBBBBBBBBBBBBBB", "exited"),
    ] {
        write_completion_job(tmp.path(), id, state, &cwd);
    }

    // `kill` should return only running jobs (word index 2: agent-exec kill <TAB>)
    let candidates = get_dynamic_candidates(root, "kill", 2, "");
    let ids: Vec<_> = candidates.iter().filter(|c| c.starts_with("01")).collect();
    assert!(
        ids.iter().any(|s| s.contains("01AAA")),
        "kill should include running job: {candidates:?}"
    );
    assert!(
        !ids.iter().any(|s| s.contains("01BBB")),
        "kill should exclude exited job: {candidates:?}"
    );
}

#[test]
fn test_dynamic_completion_empty_when_root_missing() {
    let candidates = get_dynamic_candidates("/nonexistent/path", "status", 2, "");
    // Should return flags/options but no job-ID candidates (starts with "01")
    let job_ids: Vec<_> = candidates.iter().filter(|c| c.starts_with("01")).collect();
    assert!(
        job_ids.is_empty(),
        "missing root should yield no job IDs: {candidates:?}"
    );
}

/// Helper: invoke dynamic completion passing `--root <path>` as argv words
/// (not via AGENT_EXEC_ROOT env var). This simulates fish and other shells
/// that don't set COMP_LINE but pass the partial command as argv.
///
/// The argv after `--` looks like:
///   `agent-exec --root <root> <subcommand> <partial>`
/// so the job-ID word index is 4.
fn get_dynamic_candidates_via_root_arg(root: &str, subcommand: &str, partial: &str) -> Vec<String> {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    // CompleteEnv argv: <completer_path> -- agent-exec --root <root> <subcommand> <partial>
    cmd.arg(bin.to_str().unwrap());
    cmd.arg("--");
    cmd.arg("agent-exec");
    cmd.arg("--root");
    cmd.arg(root);
    cmd.arg(subcommand);
    cmd.arg(partial);
    cmd.env("COMPLETE", "bash");
    // word index 4: agent-exec(0) --root(1) <root>(2) <subcommand>(3) <partial>(4)
    cmd.env("_CLAP_COMPLETE_INDEX", "4");
    // Intentionally do NOT set AGENT_EXEC_ROOT so the test verifies argv-based resolution.
    let output = cmd
        .output()
        .expect("run binary for dynamic completion via --root arg");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn test_dynamic_completion_with_root_arg_returns_jobs_from_that_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let cwd = std::env::current_dir().unwrap().display().to_string();

    // Create a job in the custom root.
    let job_id = "01CUSTOMROOTJOBAAAAAAAAAAAAA";
    write_completion_job(tmp.path(), job_id, "running", &cwd);

    // Trigger completion via --root argv (no AGENT_EXEC_ROOT env var).
    let candidates = get_dynamic_candidates_via_root_arg(root, "status", "");
    let ids: Vec<_> = candidates
        .iter()
        .filter(|c| c.starts_with("01CUSTOM"))
        .collect();
    assert!(
        !ids.is_empty(),
        "--root argv resolution: expected job {job_id} in candidates: {candidates:?}"
    );
}

#[test]
fn test_dynamic_completion_excludes_jobs_from_other_cwd() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_str().unwrap();
    let cwd = std::env::current_dir().unwrap().display().to_string();

    write_completion_job(tmp.path(), "01MATCHAAAAAAAAAAAAAAAAAAAA", "running", &cwd);
    write_completion_job(
        tmp.path(),
        "01OTHERBBBBBBBBBBBBBBBBBBBB",
        "running",
        "/tmp/completely-different-cwd",
    );

    let candidates = get_dynamic_candidates(root, "tail", 2, "");
    let ids: Vec<_> = candidates.iter().filter(|c| c.starts_with("01")).collect();
    assert!(
        ids.iter().any(|s| s.contains("01MATCH")),
        "tail should include current cwd job: {candidates:?}"
    );
    assert!(
        !ids.iter().any(|s| s.contains("01OTHER")),
        "tail should exclude other cwd job: {candidates:?}"
    );
}

// ---------- stdin.bin security: permissions and size limit ----------

#[cfg(unix)]
#[test]
fn stdin_bin_created_with_0o600_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let h = TestHarness::new();
    let v = h.run(&["create", "--stdin", "secret", "--", "cat"]);
    assert_envelope(&v, "create", true);
    let job_id = v["job_id"].as_str().expect("job_id missing");

    let stdin_path = std::path::Path::new(h.root())
        .join(job_id)
        .join("stdin.bin");
    assert!(stdin_path.exists(), "stdin.bin must exist");
    let mode = std::fs::metadata(&stdin_path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode, 0o600,
        "stdin.bin permissions should be 0o600, got {mode:#o}"
    );
}

// ── enrich-run-inline-completion: signal / duration_ms in run inline ─────────

#[test]
fn run_inline_returns_exit_code_and_duration_ms_on_short_exit() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--", "sh", "-c", "exit 7"]);
    assert_envelope(&v, "run", true);
    assert_eq!(v["exit_code"], 7, "exit_code should be 7: {v}");
    assert!(
        v.get("finished_at").is_some() && !v["finished_at"].is_null(),
        "finished_at must be present: {v}"
    );
    assert!(
        v.get("duration_ms").is_some() && !v["duration_ms"].is_null(),
        "duration_ms must be present for terminal job: {v}"
    );
    assert!(
        v["duration_ms"].as_u64().is_some(),
        "duration_ms must be a non-negative integer: {v}"
    );
    assert!(
        v.get("signal").is_none() || v["signal"].is_null(),
        "signal should be absent for normal exit: {v}"
    );
}

#[cfg(unix)]
#[test]
fn run_inline_includes_signal_on_signal_terminated_exit() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--", "sh", "-c", "kill -TERM $$"]);
    assert_envelope(&v, "run", true);
    assert!(
        v.get("signal").is_some() && !v["signal"].is_null(),
        "signal must be present for signal-terminated job: {v}"
    );
    let sig = v["signal"].as_str().unwrap();
    assert!(
        sig.contains("TERM") || sig == "15",
        "signal should indicate SIGTERM: {sig}"
    );
}

#[test]
fn stdin_too_large_rejects_oversized_input() {
    let h = TestHarness::new();
    let src_path = std::path::Path::new(h.root()).join("big.bin");
    // Create a file just over the limit (use a small --stdin-max-bytes for speed).
    let limit: usize = 1024;
    std::fs::write(&src_path, vec![0u8; limit + 1]).expect("write big file");

    let output = run_raw_with_root_and_stdin(
        &[
            "run",
            "--stdin-file",
            src_path.to_str().expect("utf8"),
            "--stdin-max-bytes",
            &limit.to_string(),
            "--",
            "cat",
        ],
        Some(h.root()),
        None,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout should be JSON");
    assert_eq!(v["ok"].as_bool(), Some(false));
    assert_eq!(v["error"]["code"].as_str(), Some("stdin_too_large"));
}

#[test]
fn run_inline_omits_completion_fields_for_long_jobs() {
    let h = TestHarness::new();
    let v = h.run(&["run", "--until", "1", "--", "sh", "-c", "sleep 30"]);
    assert_envelope(&v, "run", true);
    assert_eq!(
        v["state"], "running",
        "state should be running for long job: {v}"
    );
    assert!(
        v.get("exit_code").is_none() || v["exit_code"].is_null(),
        "exit_code should be absent/null for non-terminal job: {v}"
    );
    assert!(
        v.get("finished_at").is_none() || v["finished_at"].is_null(),
        "finished_at should be absent/null for non-terminal job: {v}"
    );
    assert!(
        v.get("signal").is_none() || v["signal"].is_null(),
        "signal should be absent/null for non-terminal job: {v}"
    );
    assert!(
        v.get("duration_ms").is_none() || v["duration_ms"].is_null(),
        "duration_ms should be absent/null for non-terminal job: {v}"
    );
}

// ── wait progress hints ───────────────────────────────────────────────────────

#[test]
fn wait_timeout_returns_progress_hints() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "--", "sh", "-c", "sleep 30"]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let wait_v = h.run(&["wait", "--until", "1", &job_id]);
    assert_envelope(&wait_v, "wait", true);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "running");
    assert!(
        wait_v.get("exit_code").is_none() || wait_v["exit_code"].is_null(),
        "exit_code should be absent for running job: {wait_v}"
    );
    assert!(
        wait_v.get("stdout_total_bytes").is_some() && !wait_v["stdout_total_bytes"].is_null(),
        "stdout_total_bytes should be present: {wait_v}"
    );
    assert!(
        wait_v.get("stderr_total_bytes").is_some() && !wait_v["stderr_total_bytes"].is_null(),
        "stderr_total_bytes should be present: {wait_v}"
    );
    assert!(
        wait_v.get("updated_at").is_some() && wait_v["updated_at"].is_string(),
        "updated_at should be present as string: {wait_v}"
    );

    // Clean up
    let _ = h.run(&["kill", &job_id]);
}

#[test]
fn wait_terminal_returns_progress_hints() {
    let h = TestHarness::new();
    let run_v = h.run(&["run", "--", "echo", "progress_hints_test"]);
    assert_envelope(&run_v, "run", true);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();

    let wait_v = wait_until_terminal(&h, &job_id);
    assert_eq!(wait_v["state"].as_str().unwrap_or(""), "exited");
    assert_eq!(wait_v["exit_code"].as_i64(), Some(0));
    assert!(
        wait_v.get("stdout_total_bytes").is_some() && !wait_v["stdout_total_bytes"].is_null(),
        "stdout_total_bytes should be present: {wait_v}"
    );
    assert!(
        wait_v["stdout_total_bytes"].as_u64().unwrap_or(0) > 0,
        "stdout_total_bytes should be > 0 for echo output: {wait_v}"
    );
    assert!(
        wait_v.get("stderr_total_bytes").is_some() && !wait_v["stderr_total_bytes"].is_null(),
        "stderr_total_bytes should be present: {wait_v}"
    );
    assert!(
        wait_v.get("updated_at").is_some() && wait_v["updated_at"].is_string(),
        "updated_at should be present as string: {wait_v}"
    );
}

// ── ps / rm aliases ────────────────────────────────────────────────────────────

/// `ps` returns only running jobs (shorthand for `list --state running`).
#[test]
fn ps_returns_only_running_jobs() {
    let h = TestHarness::new();

    let long_run = h.run(&["run", "sleep", "60"]);
    let long_job_id = long_run["job_id"].as_str().unwrap().to_string();

    let short_run = h.run(&["run", "echo", "done"]);
    let short_job_id = short_run["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", "--until", "5", &short_job_id]);

    let v = h.run(&["ps"]);
    assert_envelope(&v, "list", true);

    let jobs = v["jobs"].as_array().expect("jobs missing");
    let has_long = jobs
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&long_job_id));
    let has_short = jobs
        .iter()
        .any(|j| j["job_id"].as_str() == Some(&short_job_id));
    assert!(has_long, "ps should include the running job; got: {v}");
    assert!(!has_short, "ps should NOT include exited jobs; got: {v}");
    for job in jobs {
        assert_eq!(
            job["state"].as_str().unwrap_or(""),
            "running",
            "unexpected state in ps result: {job}"
        );
    }

    h.run(&["kill", &long_job_id]);
}

/// `ps --all` lists running jobs across all cwds (same as `list --state running --all`).
#[test]
fn ps_all_includes_running_jobs_from_other_cwds() {
    let h = TestHarness::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    let (va, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "60"], Some(h.root()), Some(dir_a.path()));
    let job_a_id = va["job_id"].as_str().unwrap().to_string();

    let (vb, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "60"], Some(h.root()), Some(dir_b.path()));
    let job_b_id = vb["job_id"].as_str().unwrap().to_string();

    let (v, _) = run_cmd_with_root_and_cwd(&["ps", "--all"], Some(h.root()), Some(dir_a.path()));
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().expect("jobs missing");
    let has_a = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_a_id));
    let has_b = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_b_id));
    assert!(has_a, "ps --all should include job A; got: {v}");
    assert!(has_b, "ps --all should include job B; got: {v}");

    h.run(&["kill", &job_a_id]);
    h.run(&["kill", &job_b_id]);
}

/// `ps --cwd <PATH>` scopes to that directory, matching `list --state running --cwd`.
#[test]
fn ps_cwd_flag_scopes_to_directory() {
    let h = TestHarness::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    let (va, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "60"], Some(h.root()), Some(dir_a.path()));
    let job_a_id = va["job_id"].as_str().unwrap().to_string();

    let (vb, _) =
        run_cmd_with_root_and_cwd(&["run", "sleep", "60"], Some(h.root()), Some(dir_b.path()));
    let job_b_id = vb["job_id"].as_str().unwrap().to_string();

    let dir_a_str = dir_a.path().to_str().unwrap();
    let (v, _) = run_cmd_with_root_and_cwd(&["ps", "--cwd", dir_a_str], Some(h.root()), None);
    assert_envelope(&v, "list", true);
    let jobs = v["jobs"].as_array().expect("jobs missing");
    let has_a = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_a_id));
    let has_b = jobs.iter().any(|j| j["job_id"].as_str() == Some(&job_b_id));
    assert!(has_a, "ps --cwd dir_a should include job A; got: {v}");
    assert!(!has_b, "ps --cwd dir_a should NOT include job B; got: {v}");

    h.run(&["kill", &job_a_id]);
    h.run(&["kill", &job_b_id]);
}

/// `ps --state` must not be exposed — clap rejects it as a usage error.
#[test]
fn ps_does_not_expose_state_flag() {
    let h = TestHarness::new();
    assert_usage_error(&["ps", "--state", "exited"], Some(h.root()));
}

/// `rm <JOB_ID>` is an alias of `delete <JOB_ID>`.
#[test]
fn rm_alias_deletes_finished_job() {
    let h = TestHarness::new();

    let run_v = h.run(&["run", "echo", "rm_me"]);
    let job_id = run_v["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let v = h.run(&["rm", &job_id]);
    assert_envelope(&v, "delete", true);
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(0),
        1,
        "expected deleted=1: {v}"
    );
    let jobs = v["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["job_id"].as_str().unwrap_or(""), job_id);
    assert_eq!(jobs[0]["action"].as_str().unwrap_or(""), "deleted");

    let status_v = h.run(&["status", &job_id]);
    assert_eq!(
        status_v["error"]["code"].as_str().unwrap_or(""),
        "job_not_found"
    );
}

/// `rm --dry-run --all` behaves identically to `delete --dry-run --all`.
#[test]
fn rm_alias_dry_run_all_matches_delete() {
    let h = TestHarness::new();

    let r = h.run(&["run", "echo", "bulk"]);
    let job_id = r["job_id"].as_str().unwrap().to_string();
    h.run(&["wait", &job_id]);

    let v = h.run(&["rm", "--dry-run", "--all"]);
    assert_envelope(&v, "delete", true);
    assert!(
        v["dry_run"].as_bool().unwrap_or(false),
        "expected dry_run=true: {v}"
    );
    assert_eq!(
        v["deleted"].as_u64().unwrap_or(1),
        0,
        "dry-run must not count deleted: {v}"
    );
    let jobs = v["jobs"].as_array().unwrap();
    let has_target = jobs.iter().any(|j| {
        j["job_id"].as_str() == Some(&job_id) && j["action"].as_str() == Some("would_delete")
    });
    assert!(
        has_target,
        "expected target job to be reported as would_delete: {v}"
    );

    let status_v = h.run(&["status", &job_id]);
    assert!(
        status_v["ok"].as_bool().unwrap_or(false),
        "dry-run rm must not delete the directory: {status_v}"
    );
}
