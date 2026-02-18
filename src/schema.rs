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
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        ErrorResponse {
            schema_version: SCHEMA_VERSION,
            ok: false,
            kind: "error",
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
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
    pub stdout: String,
    pub stderr: String,
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
    pub encoding: String,
}

// ---------- Persisted job metadata / state ----------

/// Nested `job` block within `meta.json`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobMetaJob {
    pub id: String,
}

/// Persisted in `meta.json` at job creation time.
///
/// Structure:
/// ```json
/// {
///   "job": { "id": "..." },
///   "schema_version": "0.1",
///   "command": [...],
///   "created_at": "...",
///   "root": "...",
///   "env_keys": [...]
/// }
/// ```
///
/// `env_keys` stores only the names (keys) of environment variables passed via `--env`.
/// Values MUST NOT be stored to avoid leaking secrets.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobMeta {
    pub job: JobMetaJob,
    pub schema_version: String,
    pub command: Vec<String>,
    pub created_at: String,
    pub root: String,
    /// Keys of environment variables provided at job creation time.
    /// Values are intentionally omitted for security.
    pub env_keys: Vec<String>,
}

impl JobMeta {
    /// Convenience accessor: returns the job ID.
    pub fn job_id(&self) -> &str {
        &self.job.id
    }
}

/// Nested `job` block within `state.json`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobStateJob {
    pub id: String,
    pub status: JobStatus,
    pub started_at: String,
}

/// Nested `result` block within `state.json`.
///
/// Option fields are serialized as `null` (not omitted) so callers always
/// see consistent keys regardless of job lifecycle stage.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobStateResult {
    /// `null` while running; set to exit code when process ends.
    pub exit_code: Option<i32>,
    /// `null` unless the process was killed by a signal.
    pub signal: Option<String>,
    /// `null` while running; set to elapsed milliseconds when process ends.
    pub duration_ms: Option<u64>,
}

/// Persisted in `state.json`, updated as the job progresses.
///
/// Structure:
/// ```json
/// {
///   "job": { "id": "...", "status": "running", "started_at": "..." },
///   "result": { "exit_code": null, "signal": null, "duration_ms": null },
///   "updated_at": "..."
/// }
/// ```
///
/// Required fields per spec: `job.id`, `job.status`, `job.started_at`,
/// `result.exit_code`, `result.signal`, `result.duration_ms`, `updated_at`.
/// Option fields MUST be serialized as `null` (not omitted) so callers always
/// see consistent keys regardless of job lifecycle stage.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobState {
    pub job: JobStateJob,
    pub result: JobStateResult,
    /// Process ID (not part of the public spec; omitted when not available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Finish time (not part of the nested result block; kept for internal use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Last time this state was written to disk (RFC 3339).
    pub updated_at: String,
}

impl JobState {
    /// Convenience accessor: returns the job ID.
    pub fn job_id(&self) -> &str {
        &self.job.id
    }

    /// Convenience accessor: returns the job status.
    pub fn status(&self) -> &JobStatus {
        &self.job.status
    }

    /// Convenience accessor: returns the started_at timestamp.
    pub fn started_at(&self) -> &str {
        &self.job.started_at
    }

    /// Convenience accessor: returns the exit code.
    pub fn exit_code(&self) -> Option<i32> {
        self.result.exit_code
    }

    /// Convenience accessor: returns the signal name.
    pub fn signal(&self) -> Option<&str> {
        self.result.signal.as_deref()
    }

    /// Convenience accessor: returns the duration in milliseconds.
    pub fn duration_ms(&self) -> Option<u64> {
        self.result.duration_ms
    }
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
