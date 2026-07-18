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

use crate::{kill, run, schema::ErrorResponse, status, tail, wait};

#[derive(Debug)]
pub struct McpStartupConfigError;

impl std::fmt::Display for McpStartupConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS must be a non-negative integer")
    }
}

impl std::error::Error for McpStartupConfigError {}

pub async fn serve(root: Option<String>) -> Result<()> {
    let max_until_seconds = parse_max_until_seconds(
        std::env::var_os("AGENT_EXEC_MCP_MAX_UNTIL_SECONDS")
            .map(|value| value.into_string().map_err(|_| McpStartupConfigError))
            .transpose()?
            .as_deref(),
    )?;
    let service = Mcp::new(root, max_until_seconds);
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
    max_until_seconds: Option<u64>,
    tool_router: ToolRouter<Mcp>,
}

impl Mcp {
    fn new(root: Option<String>, max_until_seconds: Option<u64>) -> Self {
        Self {
            root,
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
    timeout: Option<f64>,
    until: Option<f64>,
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

fn parse_max_until_seconds(value: Option<&str>) -> Result<Option<u64>> {
    value
        .map(|value| value.parse().map_err(|_| McpStartupConfigError.into()))
        .transpose()
}

fn seconds(value: Option<f64>, name: &str, default: u64) -> Result<u64, String> {
    match value {
        None => Ok(default),
        Some(value)
            if value.is_finite()
                && value >= 0.0
                && value <= u64::MAX as f64
                && value.fract() == 0.0 =>
        {
            Ok(value as u64)
        }
        Some(_) => Err(format!("{name} must be a non-negative integer")),
    }
}

fn until_seconds(value: Option<f64>, default: u64, maximum: Option<u64>) -> Result<u64, String> {
    let until = seconds(value, "until", maximum.unwrap_or(default))?;
    if let Some(maximum) = maximum
        && until > maximum
    {
        return Err(format!(
            "until of {until} seconds exceeds AGENT_EXEC_MCP_MAX_UNTIL_SECONDS maximum of {maximum} seconds"
        ));
    }
    Ok(until)
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
    } else {
        "internal_error"
    };
    ErrorResponse::new(code, message, false)
}

#[tool_router]
impl Mcp {
    #[tool(description = "Start a managed job through the canonical agent-exec lifecycle")]
    fn run(&self, Parameters(params): Parameters<RunParams>) -> Json<Value> {
        if params.command.is_empty() || params.command.iter().any(|value| value.is_empty()) {
            return tool_error("command must be a non-empty argv array");
        }
        let timeout = match seconds(params.timeout, "timeout", 0) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let until = match until_seconds(params.until, 10, self.max_until_seconds) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        let env_vars = match env_vars(params.env) {
            Ok(value) => value,
            Err(message) => return tool_error(message),
        };
        envelope(run::run_response(run::RunOpts {
            command: params.command,
            root: self.root.as_deref(),
            cwd: params.cwd.as_deref(),
            env_vars,
            timeout_ms: timeout.saturating_mul(1000),
            until_seconds: until,
            ..Default::default()
        }))
    }

    #[tool(description = "Get managed job status")]
    fn status(&self, Parameters(params): Parameters<JobParams>) -> Json<Value> {
        envelope(status::status_response(status::StatusOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
        }))
    }

    #[tool(description = "Read bounded managed job output tails")]
    fn tail(&self, Parameters(params): Parameters<TailParams>) -> Json<Value> {
        envelope(tail::tail_response(tail::TailOpts {
            job_id: &params.job_id,
            root: self.root.as_deref(),
            tail_lines: params.lines.unwrap_or(50),
            max_bytes: params.max_bytes.unwrap_or(65_536),
            ..Default::default()
        }))
    }

    #[tool(description = "Wait for a managed job for at most the requested seconds")]
    fn wait(&self, Parameters(params): Parameters<WaitParams>) -> Json<Value> {
        let until = match until_seconds(params.until, 30, self.max_until_seconds) {
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

    #[tool(description = "Explicitly terminate a managed job with TERM")]
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

    use super::{RunParams, env_vars, parse_max_until_seconds, seconds, until_seconds};

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
    fn seconds_rejects_invalid_values() {
        assert_eq!(seconds(None, "until", 10).unwrap(), 10);
        assert!(seconds(Some(-1.0), "until", 10).is_err());
        assert!(seconds(Some(f64::NAN), "until", 10).is_err());
        assert!(seconds(Some(1.5), "until", 10).is_err());
    }

    #[test]
    fn max_until_seconds_parses_valid_environment_values() {
        assert_eq!(parse_max_until_seconds(None).unwrap(), None);
        assert_eq!(parse_max_until_seconds(Some("0")).unwrap(), Some(0));
        assert_eq!(parse_max_until_seconds(Some("55")).unwrap(), Some(55));
        for value in ["", "one", "-1", "1.5", "18446744073709551616"] {
            assert!(parse_max_until_seconds(Some(value)).is_err(), "{value:?}");
        }
    }

    #[test]
    fn until_seconds_applies_configured_maximum() {
        assert_eq!(until_seconds(None, 10, Some(55)).unwrap(), 55);
        assert_eq!(until_seconds(Some(55.0), 10, Some(55)).unwrap(), 55);
        assert!(until_seconds(Some(56.0), 10, Some(55)).is_err());
        assert_eq!(until_seconds(None, 10, None).unwrap(), 10);
        assert_eq!(until_seconds(None, 30, None).unwrap(), 30);
        assert_eq!(until_seconds(Some(56.0), 10, None).unwrap(), 56);
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
