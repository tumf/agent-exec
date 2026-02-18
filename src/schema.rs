//! Shared JSON output schema types for agent-exec v0.1.
//!
//! All stdout output is JSON only. Tracing logs go to stderr.
//! Schema version is fixed at "0.1".

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "0.1";

/// Top-level envelope used for every successful response.
#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T: Serialize> {
    pub schema_version: &'static str,
    pub ok: bool,
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(flatten)]
    pub data: T,
}

impl<T: Serialize> Response<T> {
    pub fn new(kind: &'static str, data: T) -> Self {
        Response {
            schema_version: SCHEMA_VERSION,
            ok: true,
            kind,
            data,
        }
    }

    /// Serialize to a JSON string and print to stdout.
    pub fn print(&self) {
        println!(
            "{}",
            serde_json::to_string(self).expect("JSON serialization failed")
        );
    }
}

/// Top-level envelope for error responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub schema_version: &'static str,
    pub ok: bool,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    /// Whether the caller may retry the same request and expect a different outcome.
    pub retryable: bool,
}

impl ErrorResponse {
    /// Create an error response.
    ///
    /// `retryable` should be `true` only when a transient condition (e.g. I/O
    /// contention, temporary unavailability) caused the failure and the caller
    /// is expected to succeed on a subsequent attempt without changing the
    /// request.  Use `false` for permanent failures such as "job not found" or
    /// internal logic errors.
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        ErrorResponse {
            schema_version: SCHEMA_VERSION,
            ok: false,
            kind: "error",
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                retryable,
            },
        }
    }

    pub fn print(&self) {
        println!(
            "{}",
            serde_json::to_string(self).expect("JSON serialization failed")
        );
    }
}

// ---------- Command-specific response payloads ----------

/// Response for `run` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunData {
    pub job_id: String,
    pub state: String,
    /// Environment variables passed to the job, with masked values replaced by "***".
    /// Omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars: Vec<String>,
    /// Present when `snapshot_after` elapsed before `run` returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Snapshot>,
}

/// Response for `status` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusData {
    pub job_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

/// Response for `tail` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct TailData {
    pub job_id: String,
    pub stdout_tail: String,
    pub stderr_tail: String,
    /// True when the output was truncated by tail_lines or max_bytes constraints.
    pub truncated: bool,
    pub encoding: String,
}

/// Response for `wait` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitData {
    pub job_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// Response for `kill` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct KillData {
    pub job_id: String,
    pub signal: String,
}

/// Snapshot of stdout/stderr tail at a point in time.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub stdout_tail: String,
    pub stderr_tail: String,
    /// True when the output was truncated by tail_lines or max_bytes constraints.
    pub truncated: bool,
    pub encoding: String,
}

// ---------- Persisted job metadata / state ----------

/// Persisted in `meta.json` at job creation time.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobMeta {
    pub job_id: String,
    pub schema_version: String,
    pub command: Vec<String>,
    pub started_at: String,
    pub root: String,
    /// Environment variables as KEY=VALUE strings, with masked values replaced by "***".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars: Vec<String>,
    /// Keys whose values are masked in output.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mask: Vec<String>,
}

/// Persisted in `state.json`, updated as the job progresses.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobState {
    pub state: JobStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Last time state.json was updated (RFC3339). Updated periodically by progress-every.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Exited,
    Killed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Running => "running",
            JobStatus::Exited => "exited",
            JobStatus::Killed => "killed",
            JobStatus::Failed => "failed",
        }
    }
}
