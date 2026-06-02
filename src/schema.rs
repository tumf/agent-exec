//! Shared output schema types for agent-exec v0.1.
//!
//! Stdout output is JSON by default; YAML when --yaml is set.
//! Tracing logs go to stderr.
//! Schema version is fixed at "0.1".

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag: when true, print YAML instead of JSON on stdout.
static YAML_OUTPUT: AtomicBool = AtomicBool::new(false);

/// Set the output format.  Call once from `main` before running any subcommand.
pub fn set_yaml_output(yaml: bool) {
    YAML_OUTPUT.store(yaml, Ordering::Relaxed);
}

pub const SCHEMA_VERSION: &str = "0.1";

/// Serialize `value` and print to stdout in the selected format (JSON default, YAML with --yaml).
///
/// This is the single place where stdout output is written, ensuring the
/// stdout-is-machine-readable contract is enforced uniformly across all response types.
fn print_to_stdout(value: &impl Serialize) {
    if YAML_OUTPUT.load(Ordering::Relaxed) {
        print!(
            "{}",
            serde_yaml::to_string(value).expect("YAML serialization failed")
        );
    } else {
        println!(
            "{}",
            serde_json::to_string(value).expect("JSON serialization failed")
        );
    }
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
        print_to_stdout(self);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
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
                details: None,
            },
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.error.details = Some(details);
        self
    }

    pub fn print(&self) {
        print_to_stdout(self);
    }
}

// ---------- Command-specific response payloads ----------

/// Response for `create` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateData {
    pub job_id: String,
    /// Always "created".
    pub state: String,
    /// Absolute path to stdout.log for this job.
    pub stdout_log_path: String,
    /// Absolute path to stderr.log for this job.
    pub stderr_log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionData {
    pub mode: String,
    pub applied: bool,
    pub detected_kind: String,
    pub stdout: String,
    pub stderr: String,
    pub stdout_original_bytes: u64,
    pub stderr_original_bytes: u64,
    pub stdout_compressed_bytes: u64,
    pub stderr_compressed_bytes: u64,
    pub omitted: bool,
    pub strategy: Vec<String>,
}

/// Response for `run` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunData {
    pub job_id: String,
    pub state: String,
    /// Tags assigned to this job (always present; empty array when none).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Environment variables passed to the job, with masked values replaced by "***".
    /// Omitted from JSON when empty.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars: Vec<String>,
    /// Absolute path to stdout.log for this job.
    pub stdout_log_path: String,
    /// Absolute path to stderr.log for this job.
    pub stderr_log_path: String,
    /// Wall-clock milliseconds from run/start invocation start to JSON output.
    pub elapsed_ms: u64,
    /// Time spent waiting for inline output observation.
    pub waited_ms: u64,
    /// UTF-8 lossy stdout excerpt.
    pub stdout: String,
    /// UTF-8 lossy stderr excerpt.
    pub stderr: String,
    /// Raw stdout byte range represented by `stdout` as [begin, end).
    pub stdout_range: [u64; 2],
    /// Raw stderr byte range represented by `stderr` as [begin, end).
    pub stderr_range: [u64; 2],
    /// Total bytes currently observed in stdout.log.
    pub stdout_total_bytes: u64,
    /// Total bytes currently observed in stderr.log.
    pub stderr_total_bytes: u64,
    /// Encoding contract for stdout/stderr excerpts.
    pub encoding: String,
    /// Exit code when terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Finished-at timestamp when terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    /// POSIX signal name when terminated by signal (e.g. "SIGTERM").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    /// Wall-clock milliseconds from started_at to finished_at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<CompressionData>,
}

/// Response for `status` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusData {
    pub job_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// RFC 3339 timestamp when the job was created (always present).
    pub created_at: String,
    /// RFC 3339 timestamp when the job started executing; absent for `created` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
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
    /// Absolute path to stdout.log for this job.
    pub stdout_log_path: String,
    /// Absolute path to stderr.log for this job.
    pub stderr_log_path: String,
    /// Raw stdout byte range represented by `stdout` as [begin, end).
    pub stdout_range: [u64; 2],
    /// Raw stderr byte range represented by `stderr` as [begin, end).
    pub stderr_range: [u64; 2],
    /// Total bytes currently observed in stdout.log.
    pub stdout_total_bytes: u64,
    /// Total bytes currently observed in stderr.log.
    pub stderr_total_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<CompressionData>,
}

/// Response for `wait` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct WaitData {
    pub job_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Response for `kill` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct KillData {
    pub job_id: String,
    pub signal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminated_signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_within_ms: Option<u64>,
}

/// Response for `schema` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaData {
    /// The JSON Schema format identifier (e.g. "json-schema-draft-07").
    pub schema_format: String,
    /// The JSON Schema document describing all CLI response types.
    pub schema: serde_json::Value,
    /// Timestamp when the schema file was last updated (RFC 3339).
    pub generated_at: String,
}

/// Summary of a single job, included in `list` responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct JobSummary {
    pub job_id: String,
    /// Human-facing short identifier (first 7 characters of job_id).
    pub short_job_id: String,
    /// Job state: created | running | exited | killed | failed | unknown
    pub state: String,
    /// Original command argv persisted in meta.json.
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Creation timestamp from meta.json (RFC 3339).
    pub created_at: String,
    /// Execution start timestamp; absent for `created` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Tags assigned to this job (always present; empty array when none).
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Response for `tag set` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct TagSetData {
    pub job_id: String,
    /// The new deduplicated tag list as persisted to meta.json.
    pub tags: Vec<String>,
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

/// Per-job result entry in a `gc` response.
#[derive(Debug, Serialize, Deserialize)]
pub struct GcJobResult {
    pub job_id: String,
    /// Job state as reported from state.json: running | exited | killed | failed | unknown
    pub state: String,
    /// What GC did: "deleted" | "would_delete" | "skipped"
    pub action: String,
    /// Human-readable explanation for the action.
    pub reason: String,
    /// Byte size of the job directory (0 for skipped jobs where size is not computed).
    pub bytes: u64,
}

/// Response for the `gc` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct GcData {
    /// Resolved root directory path.
    pub root: String,
    /// Whether this was a dry-run (no deletions performed).
    pub dry_run: bool,
    /// The effective retention window (e.g. "30d").
    pub older_than: String,
    /// How the retention window was determined: "default" or "flag".
    pub older_than_source: String,
    /// Number of job directories actually deleted (0 when dry_run=true).
    pub deleted: u64,
    /// Number of job directories skipped (running, unreadable, or too recent).
    /// Equals `out_of_scope + failed` for the per-job results aggregated here.
    pub skipped: u64,
    /// Number of jobs that were not candidates for deletion (e.g. running,
    /// non-terminal status, missing timestamp, retention window not satisfied).
    pub out_of_scope: u64,
    /// Number of jobs that were eligible candidates but could not be removed
    /// (delete syscall failed or post-delete existence check still saw the path).
    pub failed: u64,
    /// Total bytes freed (or would be freed in dry-run mode).
    pub freed_bytes: u64,
    /// Number of job directories scanned.
    pub scanned_dirs: u64,
    /// Number of deletion candidates selected by policy.
    pub candidate_count: u64,
    /// Per-job details.
    pub jobs: Vec<GcJobResult>,
}

/// Per-job result entry in a `delete` response.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteJobResult {
    pub job_id: String,
    /// Job state as reported from state.json: created | running | exited | killed | failed | unknown
    pub state: String,
    /// What delete did: "deleted" | "would_delete" | "skipped"
    pub action: String,
    /// Human-readable explanation for the action.
    pub reason: String,
}

/// Response for the `delete` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteData {
    /// Resolved root directory path.
    pub root: String,
    /// Whether this was a dry-run (no deletions performed).
    pub dry_run: bool,
    /// Effective cwd scope used by `--all` to decide which jobs to evaluate.
    /// Absent for single-job `delete <JOB_ID>` invocations because they are not
    /// scoped by cwd.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd_scope: Option<String>,
    /// Number of job directories actually deleted (0 when dry_run=true).
    pub deleted: u64,
    /// Number of job directories skipped.
    /// For aggregations involving per-job results, equals `out_of_scope + failed`.
    pub skipped: u64,
    /// Number of jobs that were filtered out before any deletion was attempted
    /// (cwd mismatch for `--all`, or non-terminal/state-unreadable jobs).
    pub out_of_scope: u64,
    /// Number of jobs that were targeted for deletion but the deletion did not
    /// take effect (delete syscall failed or post-delete existence check still
    /// saw the path).
    pub failed: u64,
    /// Per-job details.
    pub jobs: Vec<DeleteJobResult>,
}

// ---------- install-skills response payload ----------

/// Summary of a single installed skill, included in `install_skills` responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledSkillSummary {
    /// Skill name (directory name under `.agents/skills/`).
    pub name: String,
    /// Source type string used when the skill was installed (currently "embedded").
    pub source_type: String,
    /// Absolute path to the installed skill directory.
    pub path: String,
}

/// Response for `notify set` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct NotifySetData {
    pub job_id: String,
    /// Updated notification configuration saved to meta.json.
    pub notification: NotificationConfig,
}

/// Response for `install-skills` command.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallSkillsData {
    /// List of installed skills.
    pub skills: Vec<InstalledSkillSummary>,
    /// Whether skills were installed globally (`~/.agents/`) or locally (`./.agents/`).
    pub global: bool,
    /// Absolute path to the `.skill-lock.json` file that was updated.
    pub lock_file_path: String,
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

// ---------- Notification / completion event models ----------

/// Match type for output-match notification.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputMatchType {
    #[default]
    Contains,
    Regex,
}

/// Stream selector for output-match notification.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputMatchStream {
    Stdout,
    Stderr,
    #[default]
    Either,
}

/// Configuration for output-match notifications.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputMatchConfig {
    /// Pattern to match against output lines.
    pub pattern: String,
    /// Match type: contains (substring) or regex.
    #[serde(default)]
    pub match_type: OutputMatchType,
    /// Which stream to match: stdout, stderr, or either.
    #[serde(default)]
    pub stream: OutputMatchStream,
    /// Shell command string for command sink; executed via platform shell on match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// File path for NDJSON append sink.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// Notification configuration persisted in meta.json.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationConfig {
    /// Shell command string for command sink; executed via platform shell on completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_command: Option<String>,
    /// File path for NDJSON append sink.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_file: Option<String>,
    /// Output-match notification configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_output_match: Option<OutputMatchConfig>,
}

/// The `job.finished` event payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionEvent {
    pub schema_version: String,
    pub event_type: String,
    pub job_id: String,
    pub state: String,
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    pub stdout_log_path: String,
    pub stderr_log_path: String,
}

/// Delivery result for a single notification sink.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SinkDeliveryResult {
    pub sink_type: String,
    pub target: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub attempted_at: String,
}

/// Persisted in `completion_event.json` after terminal state is reached.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionEventRecord {
    #[serde(flatten)]
    pub event: CompletionEvent,
    pub delivery_results: Vec<SinkDeliveryResult>,
}

/// The `job.output.matched` event payload.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputMatchEvent {
    pub schema_version: String,
    pub event_type: String,
    pub job_id: String,
    pub pattern: String,
    pub match_type: String,
    pub stream: String,
    pub line: String,
    pub stdout_log_path: String,
    pub stderr_log_path: String,
}

/// Delivery record for a single output-match event; appended to `notification_events.ndjson`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputMatchEventRecord {
    #[serde(flatten)]
    pub event: OutputMatchEvent,
    pub delivery_results: Vec<SinkDeliveryResult>,
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
/// `env_vars` stores KEY=VALUE strings with masked values replaced by "***" (display only).
/// `env_vars_runtime` stores the actual (unmasked) KEY=VALUE strings used at `start` time.
///   For the `run` command, this field is empty (env vars are passed directly to the supervisor).
///   For the `create`/`start` lifecycle, this field persists the real KEY=VALUE pairs so
///   `start` can apply them without re-specifying CLI arguments.
/// `mask` stores the list of keys whose values are masked in output/metadata views.
/// `cwd` stores the effective working directory at job creation time (canonicalized).
///
/// For the `create`/`start` lifecycle, additional execution-definition fields are
/// persisted so that `start` can launch the job without re-specifying CLI arguments.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobMeta {
    pub job: JobMetaJob,
    pub schema_version: String,
    pub command: Vec<String>,
    pub created_at: String,
    pub root: String,
    /// Keys of environment variables provided at job creation time.
    pub env_keys: Vec<String>,
    /// Environment variables as KEY=VALUE strings, with masked values replaced by "***".
    /// Used for display in JSON responses and metadata views only.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars: Vec<String>,
    /// Actual (unmasked) KEY=VALUE env var pairs persisted for `start` runtime use.
    /// Only populated in the `create`/`start` lifecycle. For `run`, this is empty
    /// because env vars are passed directly to the supervisor.
    /// `--env` in the create/start lifecycle is treated as durable, non-secret configuration;
    /// use `--env-file` for values that should never be written to disk.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_vars_runtime: Vec<String>,
    /// Keys whose values are masked in output.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mask: Vec<String>,
    /// Effective working directory at job creation time (canonicalized absolute path).
    /// Used by `list` to filter jobs by cwd. Absent for jobs created before this feature.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cwd: Option<String>,
    /// Notification configuration (present only when --notify-command or --notify-file was used).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub notification: Option<NotificationConfig>,
    /// User-defined tags for grouping and filtering. Empty array when none.
    #[serde(default)]
    pub tags: Vec<String>,

    // --- Execution-definition fields (persisted for create/start lifecycle) ---
    /// Whether to inherit the current process environment at start time. Default: true.
    #[serde(default = "default_inherit_env")]
    pub inherit_env: bool,
    /// Env-file paths to apply in order at start time (real values read from file on start).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub env_files: Vec<String>,
    /// Timeout in milliseconds; 0 = no timeout.
    #[serde(default)]
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    #[serde(default)]
    pub kill_after_ms: u64,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    #[serde(default)]
    pub progress_every_ms: u64,
    /// Resolved shell wrapper argv (e.g. ["sh", "-lc"]). None = resolved from config at start time.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shell_wrapper: Option<Vec<String>>,
    /// Relative path (from job directory) to materialized stdin content.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stdin_file: Option<String>,
}

fn default_inherit_env() -> bool {
    true
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
    /// RFC 3339 execution start timestamp; absent for jobs in `created` state.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub started_at: Option<String>,
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

    /// Convenience accessor: returns the started_at timestamp, if present.
    pub fn started_at(&self) -> Option<&str> {
        self.job.started_at.as_deref()
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
    Created,
    Running,
    Exited,
    Killed,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Created => "created",
            JobStatus::Running => "running",
            JobStatus::Exited => "exited",
            JobStatus::Killed => "killed",
            JobStatus::Failed => "failed",
        }
    }

    /// Returns true when the status is a non-terminal state (created or running).
    pub fn is_non_terminal(&self) -> bool {
        matches!(self, JobStatus::Created | JobStatus::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_run_data(
        exit_code: Option<i32>,
        finished_at: Option<&str>,
        signal: Option<&str>,
        duration_ms: Option<u64>,
    ) -> RunData {
        RunData {
            job_id: "abc123".into(),
            state: "exited".into(),
            tags: vec![],
            env_vars: vec![],
            stdout_log_path: "/tmp/stdout.log".into(),
            stderr_log_path: "/tmp/stderr.log".into(),
            elapsed_ms: 50,
            waited_ms: 40,
            stdout: "".into(),
            stderr: "".into(),
            stdout_range: [0, 0],
            stderr_range: [0, 0],
            stdout_total_bytes: 0,
            stderr_total_bytes: 0,
            encoding: "utf-8-lossy".into(),
            exit_code,
            finished_at: finished_at.map(|s| s.to_string()),
            signal: signal.map(|s| s.to_string()),
            duration_ms,
            compression: None,
        }
    }

    #[test]
    fn run_data_signal_and_duration_present_when_set() {
        let data = sample_run_data(
            Some(0),
            Some("2025-01-01T00:00:01Z"),
            Some("SIGTERM"),
            Some(1000),
        );
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["signal"], "SIGTERM");
        assert_eq!(json["duration_ms"], 1000);
    }

    #[test]
    fn run_data_signal_and_duration_omitted_when_none() {
        let data = sample_run_data(None, None, None, None);
        let json = serde_json::to_value(&data).unwrap();
        assert!(
            json.get("signal").is_none(),
            "signal should be omitted: {json}"
        );
        assert!(
            json.get("duration_ms").is_none(),
            "duration_ms should be omitted: {json}"
        );
        assert!(
            json.get("exit_code").is_none(),
            "exit_code should be omitted: {json}"
        );
        assert!(
            json.get("finished_at").is_none(),
            "finished_at should be omitted: {json}"
        );
    }

    #[test]
    fn run_data_signal_omitted_duration_present() {
        let data = sample_run_data(Some(7), Some("2025-01-01T00:00:01Z"), None, Some(500));
        let json = serde_json::to_value(&data).unwrap();
        assert!(json.get("signal").is_none(), "signal should be omitted");
        assert_eq!(json["duration_ms"], 500);
        assert_eq!(json["exit_code"], 7);
    }

    #[test]
    fn wait_data_progress_hints_present_when_set() {
        let data = WaitData {
            job_id: "j1".into(),
            state: "running".into(),
            exit_code: None,
            stdout_total_bytes: Some(1024),
            stderr_total_bytes: Some(256),
            updated_at: Some("2025-01-01T00:00:00Z".into()),
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["stdout_total_bytes"], 1024);
        assert_eq!(json["stderr_total_bytes"], 256);
        assert_eq!(json["updated_at"], "2025-01-01T00:00:00Z");
        assert!(json.get("exit_code").is_none());
    }

    #[test]
    fn wait_data_progress_hints_omitted_when_none() {
        let data = WaitData {
            job_id: "j2".into(),
            state: "running".into(),
            exit_code: None,
            stdout_total_bytes: None,
            stderr_total_bytes: None,
            updated_at: None,
        };
        let json = serde_json::to_value(&data).unwrap();
        assert!(json.get("stdout_total_bytes").is_none());
        assert!(json.get("stderr_total_bytes").is_none());
        assert!(json.get("updated_at").is_none());
    }

    #[test]
    fn wait_data_terminal_with_progress_hints() {
        let data = WaitData {
            job_id: "j3".into(),
            state: "exited".into(),
            exit_code: Some(0),
            stdout_total_bytes: Some(512),
            stderr_total_bytes: Some(0),
            updated_at: Some("2025-01-01T00:00:02Z".into()),
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["exit_code"], 0);
        assert_eq!(json["stdout_total_bytes"], 512);
        assert_eq!(json["updated_at"], "2025-01-01T00:00:02Z");
    }

    #[test]
    fn wait_data_roundtrip() {
        let data = WaitData {
            job_id: "j4".into(),
            state: "exited".into(),
            exit_code: Some(1),
            stdout_total_bytes: Some(100),
            stderr_total_bytes: Some(200),
            updated_at: Some("2025-06-01T12:00:00Z".into()),
        };
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: WaitData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.stdout_total_bytes, Some(100));
        assert_eq!(deserialized.stderr_total_bytes, Some(200));
        assert_eq!(
            deserialized.updated_at.as_deref(),
            Some("2025-06-01T12:00:00Z")
        );
    }

    #[test]
    fn run_data_roundtrip_with_all_fields() {
        let data = sample_run_data(
            Some(1),
            Some("2025-01-01T00:00:02Z"),
            Some("SIGKILL"),
            Some(2000),
        );
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: RunData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.signal.as_deref(), Some("SIGKILL"));
        assert_eq!(deserialized.duration_ms, Some(2000));
    }

    #[test]
    fn error_detail_omits_details_when_none() {
        let resp = ErrorResponse::new("test_error", "something went wrong", false);
        let json = serde_json::to_value(&resp).unwrap();
        assert!(
            json["error"].get("details").is_none(),
            "details should be omitted when None: {json}"
        );
    }

    #[test]
    fn error_detail_includes_details_when_present() {
        let resp = ErrorResponse::new("ambiguous_job_id", "ambiguous prefix", false).with_details(
            serde_json::json!({
                "candidates": ["id1", "id2"],
                "truncated": false,
            }),
        );
        let json = serde_json::to_value(&resp).unwrap();
        let details = &json["error"]["details"];
        assert!(!details.is_null(), "details must be present: {json}");
        assert_eq!(details["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(details["truncated"], false);
    }
}
