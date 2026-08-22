use anyhow::{Context, Result};
use rmcp::{
    Json, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    service::ServiceExt,
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    kill, run,
    schema::{ErrorResponse, NotificationStatus},
    status, tail, wait,
};

#[derive(Debug)]
pub struct McpStartupConfigError(&'static str);

impl std::fmt::Display for McpStartupConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} must be a non-negative integer", self.0)
    }
}

impl std::error::Error for McpStartupConfigError {}

const DEFAULT_UNTIL_ENV: &str = "AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS";
const MAX_UNTIL_ENV: &str = "AGENT_EXEC_MCP_MAX_UNTIL_SECONDS";
const MAX_OBSERVATION_SECONDS: u64 = 1_000_000_000_000_000;

pub async fn serve(root: Option<String>) -> Result<()> {
    let default_until_seconds = parse_until_seconds_env(DEFAULT_UNTIL_ENV)?;
    let max_until_seconds = parse_until_seconds_env(MAX_UNTIL_ENV)?;
    let service = Mcp::new(root, default_until_seconds, max_until_seconds);
    let running = service
        .serve(rmcp::transport::io::stdio())
        .await
        .context("start MCP stdio server")?;
    running
        .waiting()
        .await
        .context("run MCP stdio server")
        .map(|_| ())
}

struct Mcp {
    root: Option<String>,
    default_until_seconds: Option<u64>,
    max_until_seconds: Option<u64>,
    tool_router: ToolRouter<Mcp>,
}

impl Mcp {
    fn new(
        root: Option<String>,
        default_until_seconds: Option<u64>,
        max_until_seconds: Option<u64>,
    ) -> Self {
        Self {
            root,
            default_until_seconds,
            max_until_seconds,
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RunParams {
    command: Vec<String>,
    cwd: Option<String>,
    env: Option<std::collections::BTreeMap<String, String>>,
    /// WARNING: Gives up on the job, terminates it, and may permanently lose
    /// unfinished results. Use `until` to stop waiting without stopping the job.
    ///
    /// Seconds after which the managed job is terminated; omitted or `null`
    /// means no limit. Requires `acknowledge_result_loss=true`.
    abandon_job_after: Option<f64>,
    /// Explicit acknowledgement that `abandon_job_after` may permanently lose
    /// unfinished results. Required whenever `abandon_job_after` is set.
    acknowledge_result_loss: Option<bool>,
    /// REMOVED launch input. Accepted only so the rejection can carry migration
    /// guidance; it never launches a job. Use `abandon_job_after` with
    /// `acknowledge_result_loss` to abandon the job, or `until` to stop waiting
    /// without stopping it.
    timeout: Option<Value>,
    /// REMOVED launch input; see `timeout`.
    timeout_ms: Option<Value>,
    until: Option<f64>,
    /// Inline UTF-8 bytes fed to the managed child's stdin. Mutually exclusive
    /// with `stdin_file`. Unlike the CLI `--stdin`, a "-" value is literal input:
    /// the MCP stdio transport carries protocol frames only and is never read as
    /// job stdin.
    stdin: Option<String>,
    /// Path to a file readable by the MCP server process (server-local, not
    /// client-local). Its bytes are snapshotted into the job directory before the
    /// child launches. Mutually exclusive with `stdin`.
    stdin_file: Option<String>,
    /// Shell command string executed once, on job completion, by the MCP server
    /// process (server-local privileged configuration, equivalent to CLI
    /// `--notify-command`). Persisted as canonical run notification metadata
    /// before the workload launches; a persisted sink is what lets the response
    /// report `notification.state="armed"`.
    notify_command: Option<String>,
    /// Path writable by the MCP server process (server-local, not client-local)
    /// that receives one NDJSON `job.finished` event per completed job. Persisted
    /// with `notify_command` before the workload launches.
    notify_file: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct JobParams {
    job_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TailParams {
    job_id: String,
    lines: Option<u64>,
    max_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct WaitParams {
    job_id: String,
    until: Option<f64>,
}

#[derive(JsonSchema)]
struct McpResponseObject {}

fn parse_until_seconds_env(name: &'static str) -> Result<Option<u64>> {
    match std::env::var_os(name) {
        None => Ok(None),
        Some(value) => {
            let value = value
                .into_string()
                .map_err(|_| McpStartupConfigError(name))?;
            parse_until_seconds_value(Some(&value), name)
        }
    }
}

fn supports_observation_duration(seconds: u64) -> bool {
    seconds <= MAX_OBSERVATION_SECONDS
        && std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(seconds))
            .is_some()
}

fn parse_until_seconds_value(value: Option<&str>, name: &'static str) -> Result<Option<u64>> {
    value
        .map(|value| {
            let seconds = value
                .parse()
                .map_err(|_| anyhow::Error::from(McpStartupConfigError(name)))?;
            supports_observation_duration(seconds)
                .then_some(seconds)
                .ok_or_else(|| McpStartupConfigError(name).into())
        })
        .transpose()
}

fn seconds(value: Option<f64>, name: &str, default: u64) -> Result<u64, String> {
    match value {
        None => Ok(default),
        Some(value)
            if value.is_finite()
                && value >= 0.0
                && value < 2_f64.powi(64)
                && value.fract() == 0.0 =>
        {
            Ok(value as u64)
        }
        Some(_) => Err(format!("{name} must be a non-negative integer")),
    }
}

fn until_seconds(
    value: Option<f64>,
    default: u64,
    configured_default: Option<u64>,
    maximum: Option<u64>,
) -> Result<u64, String> {
    let requested = seconds(value, "until", configured_default.unwrap_or(default))?;
    if !supports_observation_duration(requested) {
        return Err("until exceeds the supported observation duration".to_string());
    }
    Ok(maximum.map_or(requested, |maximum| requested.min(maximum)))
}

fn tool_error(message: impl Into<String>) -> Json<Value> {
    Json(json!({"isError": true, "message": message.into()}))
}

fn env_vars(
    env: Option<std::collections::BTreeMap<String, String>>,
) -> Result<Vec<String>, String> {
    env.unwrap_or_default()
        .into_iter()
        .map(|(key, value)| {
            if key.is_empty() || key.contains('=') || key.contains('\0') {
                return Err("env keys must be non-empty and cannot contain '=' or NUL".to_string());
            }
            if value.contains('\0') {
                return Err("env values cannot contain NUL".to_string());
            }
            Ok(format!("{key}={value}"))
        })
        .collect()
}

/// Resolve the MCP `run` stdin fields into the canonical [`run::StdinSource`].
///
/// Deliberately not [`run::resolve_stdin_source`]: that helper maps `"-"` to
/// `StdinSource::CallerStdin`, and MCP has no caller stdin stream to read. The
/// process stdin is the JSON-RPC transport, so `"-"` stays literal inline input
/// here and the two fields are rejected as a pair before any job is created.
fn stdin_source(
    stdin: Option<String>,
    stdin_file: Option<String>,
) -> Result<Option<run::StdinSource>, String> {
    match (stdin, stdin_file) {
        (Some(_), Some(_)) => Err("stdin and stdin_file cannot be used together".to_string()),
        (Some(value), None) => Ok(Some(run::StdinSource::Inline(value))),
        (None, Some(path)) => Ok(Some(run::StdinSource::File(path))),
        (None, None) => Ok(None),
    }
}

/// Validate one launch-time completion sink field.
///
/// Both sinks are server-local privileged configuration granting the same
/// capability as CLI `--notify-command` / `--notify-file`. Validation happens
/// here, before any job directory exists, so an invalid sink can never produce a
/// launched workload or a misleading armed response.
fn completion_sink(value: Option<String>, name: &str) -> Result<Option<String>, String> {
    match value {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Err(format!("{name} must be a non-empty string")),
        Some(value) if value.contains('\0') => Err(format!("{name} cannot contain NUL")),
        Some(value) => Ok(Some(value)),
    }
}

/// True while the job is still observable, i.e. completion has not been reached.
///
/// A terminal response has already delivered the outcome inline, so it never
/// needs to tell the caller to stop observing.
fn is_non_terminal(state: &str) -> bool {
    matches!(state, "created" | "running")
}

/// Read the completion-notification status back out of persisted job metadata.
///
/// Reading `meta.json` rather than the request input is what keeps the response
/// truthful: `armed` is claimed only when the supplied sink actually reached
/// canonical notification persistence before launch.
fn persisted_notification(root: Option<&str>, job_id: &str) -> Option<NotificationStatus> {
    let root = crate::jobstore::resolve_root(root);
    let meta = crate::jobstore::JobDir::open(&root, job_id)
        .ok()?
        .read_meta()
        .ok()?;
    NotificationStatus::from_persisted(meta.notification.as_ref())
}

fn envelope(result: Result<impl serde::Serialize>) -> Json<Value> {
    match result {
        Ok(value) => Json(serde_json::to_value(value).expect("response serialization")),
        Err(error) => Json(serde_json::to_value(domain_error(error)).expect("error serialization")),
    }
}

fn domain_error(error: anyhow::Error) -> ErrorResponse {
    let message = error.to_string();
    let code = if error
        .downcast_ref::<crate::jobstore::JobNotFound>()
        .is_some()
    {
        "job_not_found"
    } else if error
        .downcast_ref::<crate::jobstore::AmbiguousJobId>()
        .is_some()
    {
        "ambiguous_job_id"
    } else if error
        .downcast_ref::<crate::jobstore::InvalidJobState>()
        .is_some()
    {
        "invalid_state"
    } else if error.downcast_ref::<crate::run::StdinTooLarge>().is_some() {
        "stdin_too_large"
    } else if error
        .downcast_ref::<crate::schema::AbandonLimitConflict>()
        .is_some()
    {
        "abandon_limit_conflict"
    } else if error
        .downcast_ref::<crate::run::ResultLossNotAcknowledged>()
        .is_some()
    {
        "result_loss_not_acknowledged"
    } else if error
        .downcast_ref::<crate::run::SupervisorLaunchFailed>()
        .is_some()
    {
        "launch_failed"
    } else {
        "internal_error"
    };
    ErrorResponse::new(code, message, false)
}

#[tool_router]
impl Mcp {
    #[tool(description = "Start a managed job through the canonical agent-exec lifecycle", output_schema = rmcp::handler::server::tool::cached_schema_for_type::<McpResponseObject>())]
    fn run(&self, Parameters(params): Parameters<RunParams>) -> Json<Value> {
        if params.command.is_empty() || params.command.iter().any(|value| value.is_empty()) {
            return tool_error("command must be a non-empty argv array");
        }
        // Legacy launch inputs are rejected before anything is created, and the
        // error names both replacements so the caller can pick by intent.
        for (legacy, supplied) in [
            ("timeout", params.timeout.is_some()),
            ("timeout_ms", params.timeout_ms.is_some()),
        ] {
            if supplied {
                return tool_error(run::legacy_timeout_migration_message(
                    legacy,
                    run::API_ABANDON_INPUT,
                    run::API_ACKNOWLEDGE_INPUT,
                    run::API_UNTIL_INPUT,
                ));
            }
        }
        let abandon_job_after = match seconds(params.abandon_job_after, run::API_ABANDON_INPUT, 0) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let abandon_job_after_ms = abandon_job_after.saturating_mul(1000);
        if let Err(e) = run::validate_result_loss_acknowledgement(
            abandon_job_after_ms,
            params.acknowledge_result_loss.unwrap_or(false),
            run::API_ABANDON_INPUT,
            run::API_ACKNOWLEDGE_INPUT,
            run::API_UNTIL_INPUT,
        ) {
            return tool_error(e.to_string());
        }
        let until = match until_seconds(
            params.until,
            10,
            self.default_until_seconds,
            self.max_until_seconds,
        ) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let env_vars = match env_vars(params.env) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let stdin = match stdin_source(params.stdin, params.stdin_file) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let notify_command = match completion_sink(params.notify_command, "notify_command") {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let notify_file = match completion_sink(params.notify_file, "notify_file") {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        envelope(
            run::run_response(run::RunOpts {
                command: params.command,
                root: self.root.as_deref(),
                cwd: params.cwd.as_deref(),
                env_vars,
                abandon_job_after_ms,
                acknowledge_result_loss: true,
                until_seconds: until,
                stdin,
                stdin_max_bytes: run::DEFAULT_STDIN_MAX_BYTES,
                notify_command,
                notify_file,
                ..Default::default()
            })
            .map(|mut response| {
                if is_non_terminal(&response.data.state) {
                    response.data.notification =
                        persisted_notification(self.root.as_deref(), &response.data.job_id);
                }
                response
            }),
        )
    }

    #[tool(description = "Get managed job status", output_schema = rmcp::handler::server::tool::cached_schema_for_type::<McpResponseObject>())]
    fn status(&self, Parameters(params): Parameters<JobParams>) -> Json<Value> {
        envelope(status::status_response(status::StatusOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
        }))
    }

    #[tool(description = "Read bounded managed job output tails", output_schema = rmcp::handler::server::tool::cached_schema_for_type::<McpResponseObject>())]
    fn tail(&self, Parameters(params): Parameters<TailParams>) -> Json<Value> {
        envelope(tail::tail_response(tail::TailOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
            tail_lines: params.lines.unwrap_or(50),
            max_bytes: params.max_bytes.unwrap_or(65_536),
            ..Default::default()
        }))
    }

    #[tool(description = "Wait for a managed job for at most the requested seconds", output_schema = rmcp::handler::server::tool::cached_schema_for_type::<McpResponseObject>())]
    fn wait(&self, Parameters(params): Parameters<WaitParams>) -> Json<Value> {
        let until = match until_seconds(
            params.until,
            30,
            self.default_until_seconds,
            self.max_until_seconds,
        ) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        envelope(wait::wait_response(wait::WaitOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
            poll_seconds: 1,
            until_seconds: until,
            forever: false,
        }))
    }

    #[tool(description = "Explicitly terminate a managed job with TERM", output_schema = rmcp::handler::server::tool::cached_schema_for_type::<McpResponseObject>())]
    fn kill(&self, Parameters(params): Parameters<JobParams>) -> Json<Value> {
        envelope(kill::kill_response(kill::KillOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
            signal: "TERM",
            no_wait: false,
        }))
    }
}

#[rmcp::tool_handler]
impl ServerHandler for Mcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        DEFAULT_UNTIL_ENV, MAX_UNTIL_ENV, RunParams, completion_sink, env_vars, is_non_terminal,
        parse_until_seconds_value, seconds, stdin_source, until_seconds,
    };
    use crate::run::StdinSource;

    #[test]
    fn run_params_reject_unknown_fields() {
        assert!(
            serde_json::from_value::<RunParams>(serde_json::json!({
                "command": ["true"],
                "mask": ["SECRET"]
            }))
            .is_err()
        );
    }

    #[test]
    fn run_params_accept_optional_stdin_fields() {
        let params = serde_json::from_value::<RunParams>(serde_json::json!({
            "command": ["true"],
            "stdin": "alpha\n"
        }))
        .expect("inline stdin params");
        assert_eq!(params.stdin.as_deref(), Some("alpha\n"));
        assert_eq!(params.stdin_file, None);

        let params = serde_json::from_value::<RunParams>(serde_json::json!({
            "command": ["true"],
            "stdin_file": "/tmp/input.txt"
        }))
        .expect("file stdin params");
        assert_eq!(params.stdin, None);
        assert_eq!(params.stdin_file.as_deref(), Some("/tmp/input.txt"));
    }

    #[test]
    fn stdin_source_maps_mcp_fields_without_caller_stdin() {
        assert!(stdin_source(None, None).unwrap().is_none());
        assert!(matches!(
            stdin_source(Some("alpha".to_string()), None).unwrap(),
            Some(StdinSource::Inline(value)) if value == "alpha"
        ));
        // MCP process stdin is the JSON-RPC transport, so "-" is literal input.
        assert!(matches!(
            stdin_source(Some("-".to_string()), None).unwrap(),
            Some(StdinSource::Inline(value)) if value == "-"
        ));
        assert!(matches!(
            stdin_source(None, Some("/tmp/input.txt".to_string())).unwrap(),
            Some(StdinSource::File(path)) if path == "/tmp/input.txt"
        ));
        assert_eq!(
            stdin_source(Some("-".to_string()), Some("/tmp/input.txt".to_string())).unwrap_err(),
            "stdin and stdin_file cannot be used together"
        );
    }

    #[test]
    fn run_params_accept_optional_completion_sinks() {
        let params = serde_json::from_value::<RunParams>(serde_json::json!({
            "command": ["true"],
            "notify_command": "notify.sh done",
            "notify_file": "/tmp/events.ndjson"
        }))
        .expect("completion sink params");
        assert_eq!(params.notify_command.as_deref(), Some("notify.sh done"));
        assert_eq!(params.notify_file.as_deref(), Some("/tmp/events.ndjson"));

        let params = serde_json::from_value::<RunParams>(serde_json::json!({
            "command": ["true"]
        }))
        .expect("params without sinks");
        assert_eq!(params.notify_command, None);
        assert_eq!(params.notify_file, None);
    }

    #[test]
    fn completion_sink_rejects_unusable_values() {
        assert_eq!(completion_sink(None, "notify_command").unwrap(), None);
        assert_eq!(
            completion_sink(Some("notify.sh".to_string()), "notify_command").unwrap(),
            Some("notify.sh".to_string())
        );
        for value in ["", "   ", "\n"] {
            assert_eq!(
                completion_sink(Some(value.to_string()), "notify_command").unwrap_err(),
                "notify_command must be a non-empty string",
                "{value:?}"
            );
        }
        assert_eq!(
            completion_sink(Some("notify\0.sh".to_string()), "notify_file").unwrap_err(),
            "notify_file cannot contain NUL"
        );
    }

    #[test]
    fn only_still_observable_states_can_report_armed_notification() {
        assert!(is_non_terminal("created"));
        assert!(is_non_terminal("running"));
        for state in ["exited", "killed", "failed", "unknown"] {
            assert!(!is_non_terminal(state), "{state}");
        }
    }

    #[test]
    fn seconds_rejects_invalid_values() {
        assert_eq!(seconds(None, "until", 10).unwrap(), 10);
        assert!(seconds(Some(-1.0), "until", 10).is_err());
        assert!(seconds(Some(f64::NAN), "until", 10).is_err());
        assert!(seconds(Some(1.5), "until", 10).is_err());
        assert!(seconds(Some(18_446_744_073_709_551_616.0), "until", 10).is_err());
    }

    #[test]
    fn until_environment_values_are_independently_validated() {
        for name in [DEFAULT_UNTIL_ENV, MAX_UNTIL_ENV] {
            assert_eq!(parse_until_seconds_value(None, name).unwrap(), None);
            assert_eq!(parse_until_seconds_value(Some("0"), name).unwrap(), Some(0));
            assert_eq!(
                parse_until_seconds_value(Some("55"), name).unwrap(),
                Some(55)
            );
            for value in [
                "",
                "one",
                "-1",
                "1.5",
                "18446744073709551616",
                "18446744073709551615",
            ] {
                let error = parse_until_seconds_value(Some(value), name).unwrap_err();
                assert!(error.to_string().contains(name), "{value:?}");
            }
        }
    }

    #[test]
    fn until_seconds_selects_then_caps_requested_value() {
        for (explicit, configured_default, maximum, legacy_default, expected) in [
            (Some(20.0), Some(55), Some(60), 10, 20),
            (Some(60.0), Some(55), Some(60), 10, 60),
            (Some(100.0), Some(55), Some(60), 10, 60),
            (None, Some(55), Some(60), 10, 55),
            (None, Some(100), Some(60), 10, 60),
            (None, None, Some(5), 10, 5),
            (None, None, None, 10, 10),
            (None, None, None, 30, 30),
            (Some(100.0), None, None, 10, 100),
            (Some(0.0), Some(55), Some(0), 10, 0),
        ] {
            assert_eq!(
                until_seconds(explicit, legacy_default, configured_default, maximum).unwrap(),
                expected
            );
        }
        assert_eq!(
            until_seconds(Some(1e18), 10, None, Some(0)).unwrap_err(),
            "until exceeds the supported observation duration"
        );
    }

    #[test]
    fn env_vars_rejects_invalid_keys() {
        let mut valid = BTreeMap::new();
        valid.insert("KEY".to_string(), "value".to_string());
        assert_eq!(env_vars(Some(valid)).unwrap(), ["KEY=value"]);

        for key in ["", "KEY=VALUE", "KEY\0"] {
            let mut invalid = BTreeMap::new();
            invalid.insert(key.to_string(), "value".to_string());
            assert!(env_vars(Some(invalid)).is_err(), "{key:?}");
        }
    }
}
