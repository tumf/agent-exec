mod support;

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

use serde_json::{Value, json};
use support::{TestHarness, assert_envelope, binary};

struct McpProcess {
    child: std::process::Child,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl McpProcess {
    fn start(root: &str) -> Self {
        Self::start_with_env(root, &[])
    }

    fn start_with_env(root: &str, env: &[(&str, &str)]) -> Self {
        let mut command = Command::new(binary());
        command
            .args(["--root", root, "mcp"])
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("spawn MCP server");
        Self {
            stdout: BufReader::new(child.stdout.take().expect("stdout")),
            child,
        }
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        writeln!(
            self.child.stdin.as_mut().expect("stdin"),
            "{}",
            json!({
                "jsonrpc": "2.0", "id": id, "method": method, "params": params
            })
        )
        .expect("send request");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        serde_json::from_str(line.trim()).expect("JSON-RPC stdout frame")
    }

    fn initialize(&mut self) {
        let response = self.request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "integration", "version": "1" }
            }),
        );
        assert_eq!(response["jsonrpc"], "2.0");
        writeln!(
            self.child.stdin.as_mut().expect("stdin"),
            "{}",
            json!({
                "jsonrpc": "2.0", "method": "notifications/initialized", "params": {}
            })
        )
        .expect("send initialized notification");
    }

    fn close_stdin(&mut self) {
        self.child.stdin.take();
    }

    fn call(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        let response = self.request(
            id,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        response["result"]
            .get("structuredContent")
            .cloned()
            .unwrap_or(response["result"].clone())
    }
}

/// Read a job's persisted `meta.json` from an isolated harness root.
fn job_meta(root: &str, job_id: &str) -> Value {
    let path = std::path::Path::new(root).join(job_id).join("meta.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read meta.json")).expect("meta")
}

/// Read the canonical job-local stdin materialization named by `meta.stdin_file`.
fn job_stdin_bytes(root: &str, job_id: &str) -> Vec<u8> {
    let meta = job_meta(root, job_id);
    let name = meta["stdin_file"].as_str().expect("meta.stdin_file");
    std::fs::read(std::path::Path::new(root).join(job_id).join(name)).expect("read stdin.bin")
}

#[test]
fn mcp_invalid_until_configuration_fails_before_serving_and_reports_to_stderr() {
    let harness = TestHarness::new();
    for name in [
        "AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS",
        "AGENT_EXEC_MCP_MAX_UNTIL_SECONDS",
    ] {
        for value in ["invalid", "18446744073709551615"] {
            let output = Command::new(binary())
                .args(["--root", harness.root(), "mcp"])
                .env(name, value)
                .output()
                .expect("run MCP server");

            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains(name));
        }
    }
}

#[cfg(unix)]
#[test]
fn mcp_non_utf8_until_configuration_fails_before_serving_and_reports_to_stderr() {
    use std::os::unix::ffi::OsStringExt;

    let harness = TestHarness::new();
    for name in [
        "AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS",
        "AGENT_EXEC_MCP_MAX_UNTIL_SECONDS",
    ] {
        let output = Command::new(binary())
            .args(["--root", harness.root(), "mcp"])
            .env(name, std::ffi::OsString::from_vec(vec![0xff]))
            .output()
            .expect("run MCP server");

        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(name));
    }
}

#[test]
fn mcp_lists_exactly_managed_job_tools_and_runs_jobs() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let listed = mcp.request(3, "tools/list", json!({}));
    let mut names: Vec<_> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["kill", "run", "status", "tail", "wait"]);
    for tool in listed["result"]["tools"].as_array().expect("tools") {
        assert!(
            tool.get("outputSchema")
                .is_none_or(|schema| schema["type"] == "object"),
            "MCP outputSchema must be an object schema when present: {tool}"
        );
    }

    let run = mcp.call(4, "run", json!({ "command": ["echo", "hello"] }));
    assert_envelope(&run, "run", true);
    assert_eq!(run["state"], "exited");
    assert_eq!(run["stdout"], "hello\n");
    assert_eq!(run["stderr"], "");
    for field in [
        "stdout_range",
        "stderr_range",
        "stdout_total_bytes",
        "stderr_total_bytes",
        "stdout_log_path",
        "stderr_log_path",
    ] {
        assert!(run.get(field).is_some(), "missing {field}");
    }
    let job_id = run["job_id"].as_str().expect("job id");
    assert!(
        std::path::Path::new(harness.root())
            .join(job_id)
            .join("meta.json")
            .exists()
    );
    for field in ["stdout_log_path", "stderr_log_path"] {
        assert!(std::path::Path::new(run[field].as_str().expect("log path")).exists());
    }
    assert_envelope(&harness.run(&["status", job_id]), "status", true);
}

#[test]
#[ignore = "heavy: verifies the required one-second bounded wait deadline"]
fn heavy_mcp_wait_and_tail_preserve_running_job_semantics() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({
            "command": ["sh", "-c", "printf 'first\\nsecond\\n'; sleep 3"], "until": 0
        }),
    );
    let job_id = run["job_id"].as_str().expect("job id").to_string();

    let wait = mcp.call(4, "wait", json!({ "job_id": job_id, "until": 1 }));
    assert_envelope(&wait, "wait", true);
    assert!(matches!(
        wait["state"].as_str(),
        Some("created" | "running")
    ));
    assert!(wait.get("exit_code").is_none());
    let status = mcp.call(5, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert!(matches!(
        status["state"].as_str(),
        Some("created" | "running")
    ));

    let tail = mcp.call(
        6,
        "tail",
        json!({ "job_id": job_id, "lines": 1, "max_bytes": 128 }),
    );
    assert_envelope(&tail, "tail", true);
    assert_eq!(tail["stdout"], "second\n");
    assert!(tail["stdout"].as_str().expect("stdout").len() <= 128);
    for field in [
        "stdout_range",
        "stderr_range",
        "stdout_total_bytes",
        "stderr_total_bytes",
    ] {
        assert!(tail.get(field).is_some(), "missing {field}");
    }

    let kill = mcp.call(7, "kill", json!({ "job_id": job_id }));
    assert_envelope(&kill, "kill", true);
    assert_eq!(harness.run(&["status", &job_id])["state"], "killed");
}

#[test]
fn mcp_disconnect_does_not_cancel_a_managed_job() {
    let harness = TestHarness::new();
    let job_id = {
        let mut mcp = McpProcess::start(harness.root());
        mcp.initialize();
        let run = mcp.call(
            3,
            "run",
            json!({ "command": ["sh", "-c", "sleep 1; echo done"], "until": 0 }),
        );
        let job_id = run["job_id"].as_str().expect("job id").to_string();
        mcp.close_stdin();
        job_id
    };
    let status = harness.run(&["status", &job_id]);
    assert_envelope(&status, "status", true);
    assert!(matches!(
        status["state"].as_str(),
        Some("created" | "running" | "exited")
    ));
    let waited = harness.run(&["wait", &job_id, "--until", "2"]);
    assert_envelope(&waited, "wait", true);
    assert_eq!(waited["state"], "exited");
}

#[test]
fn mcp_without_until_budget_preserves_legacy_defaults_and_explicit_values() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({
            "command": ["sh", "-c", "printf 'mcp output\\n'; printf 'mcp error\\n' >&2"],
            "until": 56
        }),
    );
    assert_envelope(&run, "run", true);
    let job_id = run["job_id"].as_str().expect("job id");
    let wait = mcp.call(4, "wait", json!({ "job_id": job_id }));
    assert_envelope(&wait, "wait", true);
    assert_eq!(wait["state"], "exited");
    assert_eq!(wait["stdout"].as_str(), Some("mcp output\n"));
    assert_eq!(wait["stderr"].as_str(), Some("mcp error\n"));
    assert_eq!(wait["encoding"].as_str(), Some("utf-8-lossy"));
    assert_eq!(wait["stdout_range"], json!([0, 11]));
    assert_eq!(wait["stderr_range"], json!([0, 10]));
    assert_eq!(wait["stdout_total_bytes"].as_u64(), Some(11));
    assert_eq!(wait["stderr_total_bytes"].as_u64(), Some(10));
}

#[test]
fn mcp_maximum_caps_over_cap_run_and_preserves_detached_job() {
    let harness = TestHarness::new();
    let mut mcp =
        McpProcess::start_with_env(harness.root(), &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")]);
    mcp.initialize();

    let run = mcp.call(
        3,
        "run",
        json!({ "command": ["sh", "-c", "sleep 1"], "until": 100 }),
    );
    assert_envelope(&run, "run", true);
    assert!(matches!(run["state"].as_str(), Some("created" | "running")));
    let job_id = run["job_id"].as_str().expect("job id");
    assert!(
        std::path::Path::new(harness.root())
            .join(job_id)
            .join("meta.json")
            .exists()
    );
}

#[test]
fn mcp_maximum_caps_over_cap_wait_without_altering_job() {
    let harness = TestHarness::new();
    let mut mcp =
        McpProcess::start_with_env(harness.root(), &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")]);
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({ "command": ["sh", "-c", "sleep 2"], "until": 0 }),
    );
    let job_id = run["job_id"].as_str().expect("job id").to_string();

    let wait = mcp.call(4, "wait", json!({ "job_id": job_id, "until": 100 }));
    assert_envelope(&wait, "wait", true);
    assert!(matches!(
        wait["state"].as_str(),
        Some("created" | "running")
    ));
    let status = mcp.call(5, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert!(matches!(
        status["state"].as_str(),
        Some("created" | "running")
    ));
    let kill = mcp.call(6, "kill", json!({ "job_id": job_id }));
    assert_envelope(&kill, "kill", true);
}

#[test]
fn mcp_default_and_maximum_configuration_are_independent() {
    let harness = TestHarness::new();
    let mut default_only = McpProcess::start_with_env(
        harness.root(),
        &[("AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS", "0")],
    );
    default_only.initialize();
    let run = default_only.call(3, "run", json!({ "command": ["sh", "-c", "sleep 1"] }));
    assert_envelope(&run, "run", true);
    assert!(matches!(run["state"].as_str(), Some("created" | "running")));
    let job_id = run["job_id"].as_str().expect("job id").to_string();
    let wait = default_only.call(4, "wait", json!({ "job_id": job_id }));
    assert_envelope(&wait, "wait", true);
    assert!(matches!(
        wait["state"].as_str(),
        Some("created" | "running")
    ));

    let max_only_harness = TestHarness::new();
    let mut max_only = McpProcess::start_with_env(
        max_only_harness.root(),
        &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")],
    );
    max_only.initialize();
    let capped_run = max_only.call(3, "run", json!({ "command": ["sh", "-c", "sleep 1"] }));
    assert_envelope(&capped_run, "run", true);
    assert!(matches!(
        capped_run["state"].as_str(),
        Some("created" | "running")
    ));
}

#[test]
fn mcp_rejects_invalid_input_before_clamping_or_creating_a_job() {
    let harness = TestHarness::new();
    let mut mcp =
        McpProcess::start_with_env(harness.root(), &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")]);
    mcp.initialize();
    for arguments in [
        json!({ "command": [] }),
        json!({ "command": ["echo", "hello"], "env": { "": "value" } }),
        json!({ "command": ["echo", "hello"], "timeout": -1 }),
        json!({ "command": ["echo", "hello"], "until": 1.5 }),
        json!({ "command": ["echo", "hello"], "until": 1_000_000_000_000_000_000_u64 }),
        serde_json::from_str(r#"{"command":["echo","hello"],"until":18446744073709551616}"#)
            .expect("out-of-range until JSON"),
    ] {
        let result = mcp.call(3, "run", arguments);
        assert_eq!(result["isError"], true);
        assert!(
            std::fs::read_dir(harness.root())
                .expect("root")
                .next()
                .is_none()
        );
    }
    let malformed = mcp.request(
        4,
        "tools/call",
        json!({ "name": "run", "arguments": { "command": "echo hello" } }),
    );
    assert!(malformed.get("error").is_some());
    assert!(
        std::fs::read_dir(harness.root())
            .expect("root")
            .next()
            .is_none()
    );
    let run = mcp.call(
        5,
        "run",
        json!({ "command": ["sh", "-c", "sleep 1"], "until": 0 }),
    );
    assert_envelope(&run, "run", true);
    let job_id = run["job_id"].as_str().expect("job id").to_string();
    let wait = mcp.call(
        6,
        "wait",
        json!({ "job_id": job_id, "until": 1_000_000_000_000_000_000_u64 }),
    );
    assert_eq!(wait["isError"], true);
    let status = mcp.call(7, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert!(matches!(
        status["state"].as_str(),
        Some("created" | "running")
    ));
    let kill = mcp.call(8, "kill", json!({ "job_id": job_id }));
    assert_envelope(&kill, "kill", true);
}

/// Regression for the zombie accumulation observed on long-lived `agent-exec mcp`
/// servers: every finished supervisor stayed as an uncollected child of the server.
/// The server here stays alive across several short jobs, so its own process-table
/// children are the evidence. Disconnect semantics are unchanged and stay covered
/// by `mcp_disconnect_does_not_cancel_a_managed_job`.
#[cfg(unix)]
#[test]
fn mcp_reaps_finished_supervisors() {
    use std::time::Duration;
    use support::proc_table::{poll_until, zombie_children};

    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let mut job_ids = Vec::new();
    for id in 3..6 {
        let run = mcp.call(
            id,
            "run",
            json!({ "command": ["echo", "reap"], "until": 0 }),
        );
        assert_envelope(&run, "run", true);
        job_ids.push(run["job_id"].as_str().expect("job id").to_string());
    }

    // The supervisors must have exited before their absence proves anything, so
    // settle terminal state off the persisted contract instead of inline waiting.
    let terminal = |job_id: &str| {
        let path = std::path::Path::new(harness.root())
            .join(job_id)
            .join("state.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .is_some_and(|state| state["job"]["status"] == "exited")
    };
    assert!(
        poll_until(Duration::from_secs(2), || job_ids
            .iter()
            .all(|job_id| terminal(job_id))),
        "short MCP jobs did not reach terminal state"
    );

    let mcp_pid = mcp.child.id();
    assert!(
        poll_until(Duration::from_millis(500), || zombie_children(mcp_pid)
            .is_empty()),
        "MCP server {mcp_pid} still owns unreaped supervisor children: {:?}",
        zombie_children(mcp_pid)
    );

    // Reaping must not block the server thread or corrupt the stdio JSON-RPC stream.
    for (offset, job_id) in job_ids.iter().enumerate() {
        let status = mcp.call(10 + offset as u64, "status", json!({ "job_id": job_id }));
        assert_envelope(&status, "status", true);
        assert_eq!(status["state"], "exited");
    }
}

#[test]
fn mcp_run_schema_exposes_optional_stdin_fields() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let listed = mcp.request(3, "tools/list", json!({}));
    let run = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "run")
        .expect("run tool")
        .clone();
    let schema = &run["inputSchema"];

    // Unknown definition-time controls stay rejected by the generated schema.
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(schema["required"], json!(["command"]));
    for field in ["stdin", "stdin_file"] {
        let property = &schema["properties"][field];
        assert_eq!(property["type"], "string", "{field} must be a string field");
        assert!(
            !schema["required"]
                .as_array()
                .expect("required")
                .contains(&json!(field)),
            "{field} must stay optional"
        );
        assert!(
            property["description"].as_str().is_some_and(|text| {
                text.contains("stdin_file") || text.contains("server-local")
            }),
            "{field} must document its semantics: {property}"
        );
    }
    // The path is resolved by the MCP server process, not the client.
    assert!(
        schema["properties"]["stdin_file"]["description"]
            .as_str()
            .expect("stdin_file description")
            .contains("server-local")
    );
}

#[test]
fn mcp_run_accepts_inline_stdin_through_the_canonical_lifecycle() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({ "command": ["cat"], "stdin": "alpha\nbeta\n" }),
    );
    assert_envelope(&run, "run", true);
    assert_eq!(run["state"], "exited");
    assert_eq!(run["stdout"], "alpha\nbeta\n");
    assert_eq!(run["stderr"], "");

    let job_id = run["job_id"].as_str().expect("job id");
    assert_eq!(job_meta(harness.root(), job_id)["stdin_file"], "stdin.bin");
    assert_eq!(job_stdin_bytes(harness.root(), job_id), b"alpha\nbeta\n");
}

#[test]
fn mcp_run_treats_inline_dash_as_literal_input() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(3, "run", json!({ "command": ["cat"], "stdin": "-" }));
    assert_envelope(&run, "run", true);
    assert_eq!(run["state"], "exited");
    assert_eq!(run["stdout"], "-");

    let job_id = run["job_id"].as_str().expect("job id");
    assert_eq!(job_stdin_bytes(harness.root(), job_id), b"-");

    // The server never waited for a second caller-stdin stream: it is still
    // serving JSON-RPC on the same transport.
    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
}

#[test]
fn mcp_run_snapshots_a_server_local_stdin_file() {
    let harness = TestHarness::new();
    let inputs = tempfile::tempdir().expect("input dir");
    let source = inputs.path().join("input.txt");
    std::fs::write(&source, b"snapshot bytes\n").expect("write stdin source");

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({ "command": ["cat"], "stdin_file": source.to_str().expect("utf-8 path") }),
    );
    assert_envelope(&run, "run", true);
    assert_eq!(run["state"], "exited");
    assert_eq!(run["stdout"], "snapshot bytes\n");

    let job_id = run["job_id"].as_str().expect("job id");
    assert_eq!(job_meta(harness.root(), job_id)["stdin_file"], "stdin.bin");
    assert_eq!(job_stdin_bytes(harness.root(), job_id), b"snapshot bytes\n");

    // The job owns a copy: later source edits cannot alter the job input.
    std::fs::write(&source, b"mutated after launch\n").expect("mutate stdin source");
    assert_eq!(job_stdin_bytes(harness.root(), job_id), b"snapshot bytes\n");
}

#[test]
fn mcp_run_rejects_conflicting_stdin_without_creating_job() {
    let harness = TestHarness::new();
    let inputs = tempfile::tempdir().expect("input dir");
    let source = inputs.path().join("input.txt");
    std::fs::write(&source, b"unused\n").expect("write stdin source");

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let result = mcp.call(
        3,
        "run",
        json!({
            "command": ["cat"],
            "stdin": "inline",
            "stdin_file": source.to_str().expect("utf-8 path")
        }),
    );
    assert_eq!(result["isError"], true);
    assert!(
        result["message"]
            .as_str()
            .is_some_and(|message| message.contains("stdin_file")),
        "conflict message must name the conflicting fields: {result}"
    );
    assert!(
        std::fs::read_dir(harness.root())
            .expect("root")
            .next()
            .is_none(),
        "a rejected stdin definition must not create a job"
    );

    // The rejection is protocol-safe: the same session keeps serving tools.
    let run = mcp.call(4, "run", json!({ "command": ["echo", "ok"] }));
    assert_envelope(&run, "run", true);
    assert_eq!(run["stdout"], "ok\n");
}

#[test]
fn mcp_run_without_stdin_keeps_null_child_stdin_and_protocol_transport() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({ "command": ["sh", "-c", "cat; printf 'eof\\n'"] }),
    );
    assert_envelope(&run, "run", true);
    assert_eq!(run["state"], "exited");
    // The child saw EOF immediately instead of consuming JSON-RPC frames.
    assert_eq!(run["stdout"], "eof\n");

    let job_id = run["job_id"].as_str().expect("job id");
    assert_eq!(job_meta(harness.root(), job_id)["stdin_file"], Value::Null);
    assert!(
        !std::path::Path::new(harness.root())
            .join(job_id)
            .join("stdin.bin")
            .exists()
    );

    // Subsequent protocol messages are still readable by the MCP server.
    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert_eq!(status["state"], "exited");
}

#[test]
fn mcp_run_rejects_unreadable_stdin_file_before_child_launch() {
    let harness = TestHarness::new();
    let inputs = tempfile::tempdir().expect("input dir");
    let missing = inputs.path().join("missing.txt");
    let unreadable = inputs.path().join("unreadable.txt");
    std::fs::write(&unreadable, b"secret\n").expect("write unreadable source");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000))
            .expect("drop read permission");
    }

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let mut candidates = vec![missing];
    // Root ignores mode bits, so only assert the unreadable case when it is real.
    #[cfg(unix)]
    if std::fs::read(&unreadable).is_err() {
        candidates.push(unreadable.clone());
    }

    for (offset, path) in candidates.iter().enumerate() {
        let result = mcp.call(
            3 + offset as u64,
            "run",
            json!({ "command": ["cat"], "stdin_file": path.to_str().expect("utf-8 path") }),
        );
        assert_envelope(&result, "error", false);
        // The failure lands before child launch: no supervisor state exists and
        // the job definition never claims a stdin materialization.
        for entry in std::fs::read_dir(harness.root()).expect("root") {
            let job_dir = entry.expect("job dir").path();
            assert!(
                !job_dir.join("state.json").exists(),
                "no supervisor may start for a failed stdin definition: {}",
                job_dir.display()
            );
            let meta: Value = serde_json::from_str(
                &std::fs::read_to_string(job_dir.join("meta.json")).expect("read meta.json"),
            )
            .expect("meta");
            assert_eq!(meta["stdin_file"], Value::Null);
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o600));
    }
}

/// MCP always uses the canonical 64 MiB `DEFAULT_STDIN_MAX_BYTES` (a configurable
/// MCP limit is deliberately out of scope), so both oversize paths have to move
/// that much data to reach the rejection. That makes this test run for seconds;
/// it stays in the default suite because the limit is a required contract.
#[test]
fn mcp_run_rejects_oversized_stdin_before_child_launch() {
    let limit = agent_exec::run::DEFAULT_STDIN_MAX_BYTES;
    let harness = TestHarness::new();
    let inputs = tempfile::tempdir().expect("input dir");
    let oversized = inputs.path().join("oversized.bin");
    std::fs::File::create(&oversized)
        .expect("create oversized source")
        .set_len(limit + 1)
        .expect("size oversized source");

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    for (offset, arguments) in [
        json!({ "command": ["cat"], "stdin_file": oversized.to_str().expect("utf-8 path") }),
        json!({ "command": ["cat"], "stdin": "a".repeat(limit as usize + 1) }),
    ]
    .into_iter()
    .enumerate()
    {
        let result = mcp.call(3 + offset as u64, "run", arguments);
        assert_envelope(&result, "error", false);
        assert_eq!(result["error"]["code"], "stdin_too_large");
        for entry in std::fs::read_dir(harness.root()).expect("root") {
            let job_dir = entry.expect("job dir").path();
            // Oversize aborts inside the bounded copy, which discards stdin.bin.
            assert!(!job_dir.join("stdin.bin").exists());
            assert!(
                !job_dir.join("state.json").exists(),
                "no supervisor may start for oversized stdin: {}",
                job_dir.display()
            );
        }
    }
}

#[test]
fn mcp_preserves_missing_job_domain_errors() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let status = mcp.call(3, "status", json!({ "job_id": "missing" }));
    assert_envelope(&status, "error", false);
    assert_eq!(status["error"]["code"], "job_not_found");
}
