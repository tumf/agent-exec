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
    assert_eq!(
        names,
        [
            "clear_abandonment",
            "kill",
            "run",
            "set_abandonment",
            "status",
            "tail",
            "wait"
        ]
    );
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

/// Read the checked-in public schema so MCP responses can be validated against
/// the same artifact the `schema` command publishes.
fn checked_in_schema() -> Value {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/agent-exec.schema.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read checked-in schema"))
        .expect("checked-in schema is valid JSON")
}

/// Start a job that outlives inline observation, so the response is returned
/// while the workload is still running: the only state where a completion hint
/// is meaningful.
fn run_detached(mcp: &mut McpProcess, id: u64, mut arguments: Value) -> Value {
    arguments["command"] = json!(["sh", "-c", "sleep 30"]);
    arguments["until"] = json!(0);
    let run = mcp.call(id, "run", arguments);
    assert_envelope(&run, "run", true);
    assert!(
        matches!(run["state"].as_str(), Some("created" | "running")),
        "job must still be observable for a completion hint: {run}"
    );
    run
}

/// The completion sink must reach canonical notification persistence before the
/// managed workload starts, and unusable sink input must be rejected before any
/// job exists at all.
#[test]
fn mcp_run_persists_completion_sink_before_launch() {
    let harness = TestHarness::new();
    let sinks = tempfile::tempdir().expect("sink dir");
    let events = sinks.path().join("events.ndjson");
    let events = events.to_str().expect("utf-8 path").to_string();

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    // Unusable sink input fails admission: no job directory, no workload.
    for (id, arguments) in [
        json!({ "command": ["true"], "notify_command": "" }),
        json!({ "command": ["true"], "notify_command": "   " }),
        json!({ "command": ["true"], "notify_file": "" }),
        json!({ "command": ["true"], "notify_command": "notify\u{0}.sh" }),
    ]
    .into_iter()
    .enumerate()
    {
        let result = mcp.call(3 + id as u64, "run", arguments);
        assert_eq!(result["isError"], true, "case {id}");
        assert!(
            result["message"]
                .as_str()
                .is_some_and(|message| message.starts_with("notify_")),
            "rejection must name the offending sink field: {result}"
        );
        assert!(
            std::fs::read_dir(harness.root())
                .expect("root")
                .next()
                .is_none(),
            "a rejected completion sink must not create a job"
        );
    }
    // An unknown notification-shaped field stays rejected by the tool schema.
    let unknown = mcp.request(
        7,
        "tools/call",
        json!({ "name": "run", "arguments": { "command": ["true"], "notify": "notify.sh" } }),
    );
    assert!(unknown.get("error").is_some(), "{unknown}");
    assert!(
        std::fs::read_dir(harness.root())
            .expect("root")
            .next()
            .is_none()
    );

    // The workload itself reads the only job's meta.json: what it printed is
    // proof that the sink was persisted before the child was launched.
    let run = mcp.call(
        8,
        "run",
        json!({
            "command": ["sh", "-c", format!("cat {}/*/meta.json", harness.root())],
            "notify_command": "true",
            "notify_file": events,
        }),
    );
    assert_envelope(&run, "run", true);
    let job_id = run["job_id"].as_str().expect("job id").to_string();
    let waited = mcp.call(9, "wait", json!({ "job_id": job_id, "until": 5 }));
    assert_envelope(&waited, "wait", true);
    assert_eq!(waited["state"], "exited");
    let tailed = mcp.call(10, "tail", json!({ "job_id": job_id }));
    assert_envelope(&tailed, "tail", true);
    let observed: Value = serde_json::from_str(tailed["stdout"].as_str().expect("stdout"))
        .expect("meta.json seen by the job");
    assert_eq!(observed["notification"]["notify_command"], "true");
    assert_eq!(observed["notification"]["notify_file"], events);

    // The same sink is the canonical persisted metadata for that job.
    let meta = job_meta(harness.root(), &job_id);
    assert_eq!(meta["notification"]["notify_command"], "true");
    assert_eq!(meta["notification"]["notify_file"], events);

    // A terminal response already delivered the outcome, so it claims nothing.
    let terminal = mcp.call(
        11,
        "run",
        json!({ "command": ["sh", "-c", "exit 0"], "notify_command": "true" }),
    );
    assert_envelope(&terminal, "run", true);
    assert_eq!(terminal["state"], "exited");
    assert!(
        terminal.get("notification").is_none(),
        "terminal responses must not claim an armed notification: {terminal}"
    );
}

/// `notification.state="armed"` is derived from persisted sink metadata, never
/// from request input or host configuration.
#[test]
fn mcp_run_reports_only_persisted_notification_as_armed() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let armed = run_detached(&mut mcp, 3, json!({ "notify_command": "true" }));
    let armed_id = armed["job_id"].as_str().expect("job id").to_string();
    assert_eq!(armed["notification"]["state"], "armed");
    assert_eq!(armed["notification"]["sinks"], json!(["command"]));
    assert_eq!(armed["notification"]["polling_required"], false);
    // The claim matches what the canonical lifecycle actually persisted.
    assert_eq!(
        job_meta(harness.root(), &armed_id)["notification"]["notify_command"],
        "true"
    );

    // The armed payload is exactly what the published schema documents.
    let mut schema = checked_in_schema();
    schema["$ref"] = json!("#/definitions/NotificationStatus");
    let validator = jsonschema::validator_for(&schema).expect("compile schema");
    assert!(
        validator.validate(&armed["notification"]).is_ok(),
        "armed notification must satisfy the public schema: {}",
        armed["notification"]
    );

    // No sink supplied: no armed claim, and nothing persisted to back one.
    let unarmed = run_detached(&mut mcp, 4, json!({}));
    let unarmed_id = unarmed["job_id"].as_str().expect("job id").to_string();
    assert!(
        unarmed.get("notification").is_none(),
        "a run without a completion sink must not claim notification: {unarmed}"
    );
    assert_eq!(
        job_meta(harness.root(), &unarmed_id)["notification"],
        Value::Null
    );

    for job_id in [armed_id, unarmed_id] {
        assert_envelope(
            &mcp.call(5, "kill", json!({ "job_id": job_id })),
            "kill",
            true,
        );
    }
}

/// The notification contract carries generic job lifecycle state only: any MCP
/// client supplies the same input and gets the same semantics back.
#[test]
fn mcp_notification_hint_is_client_independent() {
    let harness = TestHarness::new();
    let sinks = tempfile::tempdir().expect("sink dir");
    let events = sinks.path().join("events.ndjson");
    let events = events.to_str().expect("utf-8 path").to_string();

    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let command_sink = run_detached(&mut mcp, 3, json!({ "notify_command": "true" }));
    let file_sink = run_detached(&mut mcp, 4, json!({ "notify_file": events }));
    let both_sinks = run_detached(
        &mut mcp,
        5,
        json!({ "notify_command": "true", "notify_file": events }),
    );

    // Only the sink classification differs; the lifecycle semantics do not.
    assert_eq!(command_sink["notification"]["sinks"], json!(["command"]));
    assert_eq!(file_sink["notification"]["sinks"], json!(["file"]));
    assert_eq!(
        both_sinks["notification"]["sinks"],
        json!(["command", "file"])
    );
    for other in [&file_sink, &both_sinks] {
        for field in ["state", "polling_required", "message"] {
            assert_eq!(
                other["notification"][field], command_sink["notification"][field],
                "every sink class must share {field}"
            );
        }
    }

    // No client, session, chat, or originating-host concept enters the contract.
    let listed = mcp.request(6, "tools/list", json!({}));
    let run_schema = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "run")
        .expect("run tool")["inputSchema"]
        .clone();
    for surface in [
        run_schema.to_string().to_lowercase(),
        command_sink["notification"].to_string().to_lowercase(),
        file_sink["notification"].to_string().to_lowercase(),
        both_sinks["notification"].to_string().to_lowercase(),
    ] {
        for term in ["session", "chat", "originating"] {
            assert!(
                !surface.contains(term),
                "MCP notification contract must stay client-independent, found {term:?} in {surface}"
            );
        }
    }

    for run in [command_sink, file_sink, both_sinks] {
        let job_id = run["job_id"].as_str().expect("job id");
        assert_envelope(
            &mcp.call(7, "kill", json!({ "job_id": job_id })),
            "kill",
            true,
        );
    }
}

/// An agent can decide to stop observing from the response contract alone.
#[test]
fn mcp_armed_response_explains_next_action() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let armed = run_detached(&mut mcp, 3, json!({ "notify_command": "true" }));
    assert_eq!(armed["notification"]["polling_required"], false);
    let message = armed["notification"]["message"]
        .as_str()
        .expect("notification message");
    let lowercase = message.to_lowercase();
    assert!(
        lowercase.contains("completion") && lowercase.contains("sink"),
        "the message must say completion is delivered through the sink: {message}"
    );
    for command in ["wait", "status", "tail"] {
        assert!(
            lowercase.contains(command),
            "the message must name {command} as unnecessary polling: {message}"
        );
    }
    assert!(
        lowercase.contains("do not poll"),
        "the message must tell the agent not to poll: {message}"
    );

    // An unarmed job makes no such claim, and explicit observation stays available.
    let unarmed = run_detached(&mut mcp, 4, json!({}));
    let unarmed_id = unarmed["job_id"].as_str().expect("job id").to_string();
    assert!(unarmed.get("notification").is_none(), "{unarmed}");
    assert_envelope(
        &mcp.call(5, "status", json!({ "job_id": unarmed_id })),
        "status",
        true,
    );
    assert_envelope(
        &mcp.call(6, "tail", json!({ "job_id": unarmed_id })),
        "tail",
        true,
    );
    assert_envelope(
        &mcp.call(7, "wait", json!({ "job_id": unarmed_id, "until": 0 })),
        "wait",
        true,
    );

    // Explicit observation also remains available for the armed job: the hint
    // discourages polling, it does not remove the diagnosis path.
    let armed_id = armed["job_id"].as_str().expect("job id").to_string();
    assert_envelope(
        &mcp.call(8, "status", json!({ "job_id": armed_id })),
        "status",
        true,
    );

    for job_id in [armed_id, unarmed_id] {
        assert_envelope(
            &mcp.call(9, "kill", json!({ "job_id": job_id })),
            "kill",
            true,
        );
    }
}

// ── abandon-job-after ──────────────────────────────────────────────────────────

/// Extract the generated `run` input schema from `tools/list`.
fn abandon_job_after_run_schema(mcp: &mut McpProcess) -> Value {
    let listed = mcp.request(3, "tools/list", json!({}));
    listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "run")
        .expect("run tool")["inputSchema"]
        .clone()
}

fn abandon_job_after_job_dirs(root: &str) -> usize {
    std::fs::read_dir(root)
        .expect("read jobs root")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count()
}

#[test]
fn abandon_job_after_mcp_schema_leads_with_the_result_loss_warning() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let schema = abandon_job_after_run_schema(&mut mcp);

    let description = schema["properties"]["abandon_job_after"]["description"]
        .as_str()
        .expect("abandon_job_after description");
    assert!(
        description
            .starts_with("WARNING: Gives up on the job, terminates it, and may permanently lose"),
        "abandon_job_after must open with the result-loss warning: {description}"
    );
    assert!(
        description.contains("acknowledge_result_loss"),
        "abandon_job_after must name its acknowledgement: {description}"
    );
    assert!(
        schema["properties"]
            .get("acknowledge_result_loss")
            .is_some(),
        "the acknowledgement must be part of the published schema: {schema}"
    );
    assert!(
        !schema["required"]
            .as_array()
            .expect("required")
            .contains(&json!("abandon_job_after")),
        "the destructive control must stay optional: {schema}"
    );
}

#[test]
fn abandon_job_after_mcp_rejects_unacknowledged_abandonment() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    for arguments in [
        json!({ "command": ["sleep", "60"], "abandon_job_after": 1 }),
        json!({ "command": ["sleep", "60"], "abandon_job_after": 1, "acknowledge_result_loss": false }),
    ] {
        let result = mcp.call(3, "run", arguments.clone());
        assert_eq!(result["isError"], true, "{arguments}: {result}");
        let message = result["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("may permanently lose unfinished results")
                && message.contains("acknowledge_result_loss")
                && message.contains("until"),
            "{arguments}: message must warn and name both alternatives: {message}"
        );
        assert_eq!(
            abandon_job_after_job_dirs(harness.root()),
            0,
            "{arguments} must not create a job"
        );
    }
}

#[test]
fn abandon_job_after_mcp_legacy_timeout_error_is_actionable() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    for arguments in [
        json!({ "command": ["echo", "hello"], "timeout": 1 }),
        json!({ "command": ["echo", "hello"], "timeout_ms": 1000 }),
    ] {
        let result = mcp.call(3, "run", arguments.clone());
        assert_eq!(result["isError"], true, "{arguments}: {result}");
        let message = result["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("abandon_job_after") && message.contains("until"),
            "{arguments}: migration error must name both replacements: {message}"
        );
        assert_eq!(
            abandon_job_after_job_dirs(harness.root()),
            0,
            "{arguments} must not create a job"
        );
    }
}

#[test]
fn abandon_job_after_mcp_abandonment_and_observation_remain_distinct() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    // Acknowledged abandonment terminates its workload.
    let abandoned = mcp.call(
        3,
        "run",
        json!({
            "command": ["sleep", "60"],
            "abandon_job_after": 1,
            "acknowledge_result_loss": true,
            "until": 6
        }),
    );
    assert_envelope(&abandoned, "run", true);
    let abandoned_id = abandoned["job_id"].as_str().expect("job id").to_string();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let status = loop {
        let status = mcp.call(4, "status", json!({ "job_id": abandoned_id }));
        if status["state"] != "running" {
            break status;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "abandon_job_after never terminated the workload: {status}"
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    assert_eq!(status["abandoned_by"], "abandon_job_after", "{status}");
    assert_eq!(status["result_loss"], true, "{status}");

    // Observation expiry leaves its workload alone.
    let observed = mcp.call(5, "run", json!({ "command": ["sleep", "30"], "until": 1 }));
    assert_envelope(&observed, "run", true);
    assert_eq!(observed["state"], "running", "{observed}");
    let observed_id = observed["job_id"].as_str().expect("job id").to_string();
    let observed_status = mcp.call(6, "status", json!({ "job_id": observed_id }));
    assert_eq!(observed_status["state"], "running", "{observed_status}");
    assert!(
        observed_status.get("abandoned_by").is_none(),
        "observation must never mark abandonment: {observed_status}"
    );
    assert_envelope(
        &mcp.call(7, "kill", json!({ "job_id": observed_id })),
        "kill",
        true,
    );
}

// ── mutable running-job abandonment control ────────────────────────────────────

/// Launch a detached long-running managed job through MCP.
fn mutable_abandonment_launch(mcp: &mut McpProcess, id: u64) -> String {
    let launched = mcp.call(
        id,
        "run",
        json!({ "command": ["sleep", "600"], "until": 0 }),
    );
    assert_envelope(&launched, "run", true);
    launched["job_id"].as_str().expect("job_id").to_string()
}

/// MCP replaces a running deadline and reports the same effective control that
/// `status` reads back out of the durable record.
#[test]
fn mutable_abandonment_mcp_sets_and_reports_a_running_deadline() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let job_id = mutable_abandonment_launch(&mut mcp, 2);

    let set = mcp.call(
        3,
        "set_abandonment",
        json!({ "job_id": job_id, "abandon_in": 30, "acknowledge_result_loss": true }),
    );
    assert_envelope(&set, "abandon_set", true);
    assert_eq!(set["abandon_revision"].as_u64(), Some(2));
    assert_eq!(set["abandon_job_after_ms"].as_u64(), Some(30_000));
    assert_eq!(set["abandon_configured_by"].as_str(), Some("update"));
    let deadline = set["abandon_deadline"]
        .as_str()
        .expect("deadline")
        .to_string();

    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert_eq!(status["abandon_revision"].as_u64(), Some(2));
    assert_eq!(status["abandon_deadline"].as_str(), Some(deadline.as_str()));
    assert_eq!(status["abandon_configured_by"].as_str(), Some("update"));
    assert!(
        status["abandon_remaining_ms"].as_u64().is_some(),
        "{status}"
    );

    mcp.call(5, "kill", json!({ "job_id": job_id }));
}

/// MCP clears a running deadline; the workload survives it.
#[test]
fn mutable_abandonment_mcp_clears_a_running_deadline() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let launched = mcp.call(
        2,
        "run",
        json!({
            "command": ["sleep", "600"],
            "until": 0,
            "abandon_job_after": 2,
            "acknowledge_result_loss": true
        }),
    );
    assert_envelope(&launched, "run", true);
    let job_id = launched["job_id"].as_str().expect("job_id").to_string();

    let cleared = mcp.call(3, "clear_abandonment", json!({ "job_id": job_id }));
    assert_envelope(&cleared, "abandon_clear", true);
    assert_eq!(cleared["abandon_phase"].as_str(), Some("disabled"));
    assert!(cleared.get("abandon_deadline").is_none(), "{cleared}");

    std::thread::sleep(std::time::Duration::from_secs(4));
    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_eq!(
        status["state"].as_str().unwrap_or(""),
        "running",
        "a cleared deadline must not signal the job: {status}"
    );

    mcp.call(5, "kill", json!({ "job_id": job_id }));
}

/// Every MCP set requires acknowledgement, and the rejection carries the
/// result-loss warning rather than a bare validation message.
#[test]
fn mutable_abandonment_mcp_set_requires_acknowledgement() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let job_id = mutable_abandonment_launch(&mut mcp, 2);

    for arguments in [
        json!({ "job_id": job_id, "abandon_in": 30 }),
        json!({ "job_id": job_id, "abandon_in": 30, "acknowledge_result_loss": false }),
    ] {
        let rejected = mcp.call(3, "set_abandonment", arguments.clone());
        assert_eq!(
            rejected["isError"].as_bool(),
            Some(true),
            "{arguments} -> {rejected}"
        );
        let message = rejected["message"].as_str().unwrap_or("");
        assert!(
            message.contains("permanently lose"),
            "the rejection must carry the result-loss warning: {rejected}"
        );
        assert!(
            message.contains("acknowledge_result_loss"),
            "the rejection must name the field that unblocks it: {rejected}"
        );
    }

    // Nothing was mutated: the launch revision is still current.
    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_eq!(status["abandon_revision"].as_u64(), Some(1));

    mcp.call(5, "kill", json!({ "job_id": job_id }));
}

/// A running job with no supervisor-authored control record is rejected with a
/// stable job-domain error rather than reporting a change nothing will honour.
#[test]
fn mutable_abandonment_mcp_rejects_a_legacy_supervisor() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let job_id = mutable_abandonment_launch(&mut mcp, 2);
    std::fs::remove_file(
        std::path::Path::new(harness.root())
            .join(&job_id)
            .join("abandon_control.json"),
    )
    .expect("remove control record");

    for (name, arguments) in [
        (
            "set_abandonment",
            json!({ "job_id": job_id, "abandon_in": 30, "acknowledge_result_loss": true }),
        ),
        ("clear_abandonment", json!({ "job_id": job_id })),
    ] {
        let rejected = mcp.call(3, name, arguments);
        assert_envelope(&rejected, "error", false);
        assert_eq!(
            rejected["error"]["code"].as_str().unwrap_or(""),
            "invalid_state",
            "{name} -> {rejected}"
        );
        assert!(
            rejected["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("restart"),
            "{name} must direct the operator to restart: {rejected}"
        );
    }

    mcp.call(4, "kill", json!({ "job_id": job_id }));
}

/// Both tools are advertised, and the destructive one leads with the warning.
#[test]
fn mutable_abandonment_mcp_advertises_both_tools_with_the_destructive_boundary() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();

    let listed = mcp.request(2, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("tools array");
    let by_name = |name: &str| {
        tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} must be advertised: {listed}"))
            .clone()
    };

    let set = by_name("set_abandonment");
    assert!(
        set["description"]
            .as_str()
            .unwrap_or("")
            .starts_with("WARNING: Gives up on the job, terminates it, and may permanently lose"),
        "set_abandonment must lead with the result-loss warning: {set}"
    );
    let properties = &set["inputSchema"]["properties"];
    assert!(properties.get("abandon_in").is_some(), "{set}");
    assert!(properties.get("acknowledge_result_loss").is_some(), "{set}");

    let clear = by_name("clear_abandonment");
    assert!(
        clear["description"]
            .as_str()
            .unwrap_or("")
            .contains("non-destructive"),
        "clear_abandonment must say it never signals the job: {clear}"
    );
}
