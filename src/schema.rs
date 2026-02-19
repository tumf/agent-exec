//! Shared JSON output schema types for agent-exec v0.1.
//!
//! All stdout output is JSON only. Tracing logs go to stderr.
//! Schema version is fixed at "0.1".

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "0.1";

/// Serialize `value` to a JSON string and print it as a single line to stdout.
///
/// This is the single place where stdout JSON output is written, ensuring the
/// stdout-is-JSON-only contract is enforced uniformly across all response types.
fn print_json_to_stdout(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string(value).expect("JSON serialization failed")
    );
}

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
        print_json_to_stdout(self);
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
        print_json_to_stdout(self);
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
    /// Absolute path to stdout.log for this job.
    pub stdout_log_path: String,
    /// Absolute path to stderr.log for this job.
    pub stderr_log_path: String,
    /// Milliseconds actually waited for snapshot (0 when snapshot_after=0).
    pub waited_ms: u64,
    /// Wall-clock milliseconds from run invocation start to JSON output.
    pub elapsed_ms: u64,
    /// Exit code of the process; present only when `--wait` is used and job has terminated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// RFC 3339 timestamp when the job finished; present only when `--wait` is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// Final log tail snapshot taken after job completion; present only when `--wait` is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_snapshot: Option<Snapshot>,
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
    /// Absolute path to stdout.log for this job.
    pub stdout_log_path: String,
    /// Absolute path to stderr.log for this job.
    pub stderr_log_path: String,
    /// Size of stdout.log in bytes at the time of the tail read (0 if file absent).
    pub stdout_observed_bytes: u64,
    /// Size of stderr.log in bytes at the time of the tail read (0 if file absent).
    pub stderr_observed_bytes: u64,
    /// UTF-8 byte length of the stdout_tail string included in this response.
    pub stdout_included_bytes: u64,
    /// UTF-8 byte length of the stderr_tail string included in this response.
    pub stderr_included_bytes: u64,
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

/// Summary of a single job, included in `list` responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct JobSummary {
    pub job_id: String,
    /// Job state: running | exited | killed | failed | unknown
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Creation timestamp from meta.json (RFC 3339).
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Response for `list` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListData {
    /// Resolved root directory path.
    pub root: String,
    /// Array of job summaries, sorted by started_at descending.
    pub jobs: Vec<JobSummary>,
    /// True when the result was truncated by --limit.
    pub truncated: bool,
    /// Number of directories skipped because they could not be read as jobs.
    pub skipped: u64,
}

/// Snapshot of stdout/stderr tail at a point in time.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub stdout_tail: String,
    pub stderr_tail: String,
    /// True when the output was truncated by tail_lines or max_bytes constraints.
    pub truncated: bool,
    pub encoding: String,
    /// Size of stdout.log in bytes at the time of the snapshot (0 if file absent).
    pub stdout_observed_bytes: u64,
    /// Size of stderr.log in bytes at the time of the snapshot (0 if file absent).
    pub stderr_observed_bytes: u64,
    /// UTF-8 byte length of the stdout_tail string included in this snapshot.
    pub stdout_included_bytes: u64,
    /// UTF-8 byte length of the stderr_tail string included in this snapshot.
    pub stderr_included_bytes: u64,
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
///   "env_keys": [...],
///   "env_vars": [...],
///   "mask": [...]
/// }
/// ```
///
/// `env_keys` stores only the names (keys) of environment variables passed via `--env`.
/// Values MUST NOT be stored to avoid leaking secrets.
/// `env_vars` stores KEY=VALUE strings with masked values replaced by "***".
/// `mask` stores the list of keys whose values are masked.
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
    /// Environment variables as KEY=VALUE strings, with masked values replaced by "***".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars: Vec<String>,
    /// Keys whose values are masked in output.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mask: Vec<String>,
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
    /// Windows-only: name of the Job Object used to manage the process tree.
    /// Present only when the supervisor successfully created and assigned a
    /// named Job Object; absent on non-Windows platforms and when creation
    /// fails (in which case tree management falls back to snapshot enumeration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows_job_name: Option<String>,
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
