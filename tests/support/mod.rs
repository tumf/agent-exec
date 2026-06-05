#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// Path to the compiled binary.
pub fn binary() -> PathBuf {
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
pub struct TestHarness {
    /// The underlying temporary directory (kept alive for the harness lifetime).
    _tmp: tempfile::TempDir,
    /// String path to the root, set as `AGENT_EXEC_ROOT` for every command.
    root: String,
}

impl TestHarness {
    /// Create a new harness with a fresh temporary directory.
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let root = tmp
            .path()
            .to_str()
            .expect("tempdir path is valid UTF-8")
            .to_string();
        Self { _tmp: tmp, root }
    }

    /// Return the root path as a `&str`.
    pub fn root(&self) -> &str {
        &self.root
    }

    /// Run the binary with the given args under this harness's root, returning
    /// the parsed stdout JSON. Panics with a descriptive message on any error.
    pub fn run(&self, args: &[&str]) -> serde_json::Value {
        run_cmd_with_root(args, Some(&self.root))
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_raw_with_root_and_stdin(
    args: &[&str],
    root: Option<&str>,
    stdin_bytes: Option<&[u8]>,
) -> Output {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    if let Some(r) = root {
        cmd.env("AGENT_EXEC_ROOT", r);
    }
    if stdin_bytes.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("spawn binary");
    if let Some(bytes) = stdin_bytes {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(bytes).expect("write stdin bytes");
        }
    }

    child.wait_with_output().expect("wait binary output")
}

pub fn run_cmd_with_root(args: &[&str], root: Option<&str>) -> serde_json::Value {
    let output = run_raw_with_root_and_stdin(args, root, None);
    parse_json_stdout(&output, args)
}

pub fn run_cmd_with_root_and_stdin(
    args: &[&str],
    root: Option<&str>,
    stdin_bytes: &[u8],
) -> serde_json::Value {
    let output = run_raw_with_root_and_stdin(args, root, Some(stdin_bytes));
    parse_json_stdout(&output, args)
}

/// Run a command expecting a clap usage error (exit code 2, empty stdout).
///
/// Asserts that the process exits with code 2 and produces no JSON on stdout.
pub fn assert_usage_error(args: &[&str], root: Option<&str>) {
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
pub fn run_cmd_with_global_root_flag(root: &str, args: &[&str]) -> serde_json::Value {
    let bin = binary();
    let mut cmd = Command::new(&bin);
    cmd.arg("--root").arg(root);
    cmd.args(args);
    // Clear AGENT_EXEC_ROOT to ensure the CLI flag is what takes effect.
    cmd.env_remove("AGENT_EXEC_ROOT");
    let output = cmd.output().expect("run binary");
    parse_json_stdout(&output, args)
}

/// Run the binary with `--root <root>` placed after the subcommand name (legacy position).
/// Verifies backward-compatible syntax: `agent-exec <subcommand> --root <PATH> ...`.
/// Because --root is declared with `global = true`, clap accepts it in both positions.
pub fn run_cmd_with_subcommand_root_flag(
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
pub fn assert_envelope(v: &serde_json::Value, expected_type: &str, expected_ok: bool) {
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

pub fn assert_common_fields(json: &serde_json::Value) {
    assert!(
        json.get("schema_version").is_some(),
        "missing schema_version in: {json}"
    );
    assert!(json.get("ok").is_some(), "missing ok in: {json}");
    assert!(json.get("type").is_some(), "missing type in: {json}");
}

fn parse_json_stdout(output: &Output, args: &[&str]) -> serde_json::Value {
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
