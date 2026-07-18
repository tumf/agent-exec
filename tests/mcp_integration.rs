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
    let run = mcp.call(3, "run", json!({ "command": ["true"], "until": 56 }));
    assert_envelope(&run, "run", true);
    let job_id = run["job_id"].as_str().expect("job id");
    let wait = mcp.call(4, "wait", json!({ "job_id": job_id }));
    assert_envelope(&wait, "wait", true);
    assert_eq!(wait["state"], "exited");
}

#[test]
fn mcp_configured_until_budget_applies_to_run_without_creating_over_budget_jobs() {
    let harness = TestHarness::new();
    let mut mcp =
        McpProcess::start_with_env(harness.root(), &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")]);
    mcp.initialize();

    for arguments in [
        json!({ "command": ["sh", "-c", "sleep 1"] }),
        json!({ "command": ["sh", "-c", "sleep 1"], "until": 0 }),
    ] {
        let run = mcp.call(3, "run", arguments);
        assert_envelope(&run, "run", true);
        assert!(matches!(run["state"].as_str(), Some("created" | "running")));
    }

    let over_budget = mcp.call(
        4,
        "run",
        json!({ "command": ["echo", "never"], "until": 1 }),
    );
    assert_eq!(over_budget["isError"], true);
    assert!(
        over_budget["message"]
            .as_str()
            .expect("error message")
            .contains("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS")
    );
    assert_eq!(
        std::fs::read_dir(harness.root()).expect("root").count(),
        2,
        "over-budget run must not create a job"
    );
}

#[test]
fn mcp_configured_until_budget_rejects_wait_without_altering_job() {
    let harness = TestHarness::new();
    let mut mcp =
        McpProcess::start_with_env(harness.root(), &[("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS", "0")]);
    mcp.initialize();
    let run = mcp.call(3, "run", json!({ "command": ["sh", "-c", "sleep 2"] }));
    let job_id = run["job_id"].as_str().expect("job id").to_string();

    for arguments in [
        json!({ "job_id": job_id }),
        json!({ "job_id": job_id, "until": 0 }),
    ] {
        let wait = mcp.call(4, "wait", arguments);
        assert_envelope(&wait, "wait", true);
        assert!(matches!(
            wait["state"].as_str(),
            Some("created" | "running")
        ));
    }

    let over_budget = mcp.call(5, "wait", json!({ "job_id": job_id, "until": 1 }));
    assert_eq!(over_budget["isError"], true);
    let status = mcp.call(6, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    assert!(matches!(
        status["state"].as_str(),
        Some("created" | "running")
    ));
    let kill = mcp.call(7, "kill", json!({ "job_id": job_id }));
    assert_envelope(&kill, "kill", true);
}

#[test]
fn mcp_rejects_invalid_input_without_creating_a_job() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    for arguments in [
        json!({ "command": [] }),
        json!({ "command": ["echo", "hello"], "env": { "": "value" } }),
        json!({ "command": ["echo", "hello"], "timeout": -1 }),
        json!({ "command": ["echo", "hello"], "until": 1.5 }),
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
