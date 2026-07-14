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
        let mut child = Command::new(binary())
            .args(["--root", root, "mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn MCP server");
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
    let names: Vec<_> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert_eq!(names.len(), 5);
    for expected in ["run", "status", "tail", "wait", "kill"] {
        assert!(names.contains(&expected));
    }

    let run = mcp.call(4, "run", json!({ "command": ["echo", "hello"] }));
    assert_envelope(&run, "run", true);
    assert_eq!(run["stdout"], "hello\n");
    let job_id = run["job_id"].as_str().expect("job id");
    assert!(
        std::path::Path::new(harness.root())
            .join(job_id)
            .join("meta.json")
            .exists()
    );
    assert_envelope(&harness.run(&["status", job_id]), "status", true);
}

#[test]
fn mcp_observes_and_explicitly_kills_a_running_job() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let run = mcp.call(
        3,
        "run",
        json!({
            "command": ["sh", "-c", "sleep 5; echo done"], "until": 0
        }),
    );
    let job_id = run["job_id"].as_str().expect("job id").to_string();

    let status = mcp.call(4, "status", json!({ "job_id": job_id }));
    assert_envelope(&status, "status", true);
    let wait = mcp.call(5, "wait", json!({ "job_id": job_id, "until": 0 }));
    assert_envelope(&wait, "wait", true);
    assert!(wait["exit_code"].is_null());
    let tail = mcp.call(
        6,
        "tail",
        json!({ "job_id": job_id, "lines": 1, "max_bytes": 128 }),
    );
    assert_envelope(&tail, "tail", true);
    let kill = mcp.call(7, "kill", json!({ "job_id": job_id }));
    assert_envelope(&kill, "kill", true);
    assert_eq!(harness.run(&["status", &job_id])["state"], "killed");
}

#[test]
fn mcp_rejects_empty_command_without_creating_a_job() {
    let harness = TestHarness::new();
    let mut mcp = McpProcess::start(harness.root());
    mcp.initialize();
    let result = mcp.call(3, "run", json!({ "command": [] }));
    assert_eq!(result["isError"], true);
    assert!(
        std::fs::read_dir(harness.root())
            .expect("root")
            .next()
            .is_none()
    );
}
