//! Embedded managed-job API.
//!
//! This module is the contract for Rust programs that link `agent-exec` as a
//! library instead of shelling out to the `agent-exec` executable. It exposes
//! typed `run`, `status`, `tail`, `list`, and `kill` operations that return
//! ordinary Rust data. Nothing here parses command JSON, writes command JSON to
//! stdout, or spawns the public CLI.
//!
//! # Embedding startup sequence
//!
//! Managed jobs stay managed because supervision runs in a **detached process**,
//! not an in-process thread: the supervisor owns the workload's stdout/stderr,
//! timeout escalation, notifications, state updates, and process-tree cleanup
//! long after the launching call returns. Running it in-process would tie job
//! lifetime to the embedding process, so the embedded launcher always
//! re-executes a supervisor executable.
//!
//! By default that executable is the consumer's own binary, which therefore has
//! to know how to be a supervisor. That is what
//! [`delegate_supervisor_startup`] is for, and it MUST be called **before** the
//! consumer's own argument parsing:
//!
//! ```no_run
//! use agent_exec::embedded::{EmbeddedClient, RunRequest, SupervisorDelegation};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Reserved supervisor invocations are claimed here and never reach the
//!     // consumer's own CLI. Ordinary invocations fall through untouched.
//!     if agent_exec::embedded::delegate_supervisor_startup()? == SupervisorDelegation::Supervised {
//!         return Ok(());
//!     }
//!
//!     // The jobs root is always explicit for embedded callers.
//!     let client = EmbeddedClient::new("/var/lib/my-app/agent-exec-jobs")?;
//!
//!     let launched = client.run(RunRequest::new(vec!["echo hello".to_string()]))?;
//!     println!("{} -> {}", launched.job_id, launched.state);
//!
//!     let status = client.status(&launched.job_id)?;
//!     println!("state={}", status.state);
//!     Ok(())
//! }
//! ```
//!
//! ## Reserved invocation ownership
//!
//! [`SUPERVISOR_MARKER`] at `argv[1]` is a reserved dispatch token owned by this
//! crate. Delegation claims an invocation **only** on an exact match; anything
//! else returns [`SupervisorDelegation::NotSupervisor`] with the consumer's
//! arguments untouched. Once claimed, the generated argument grammar is parsed
//! strictly and missing, duplicated, malformed, or unexpected arguments fail
//! closed rather than falling through to consumer command handling. The marker
//! is a dispatch token, not an authentication boundary: delegated supervision
//! additionally validates the explicit root and job identity against the
//! pre-created metadata before acknowledging startup. The marker and its
//! argument grammar are private and are not a stable end-user CLI.
//!
//! ## Explicit root and supervisor executable
//!
//! [`EmbeddedClient::new`] requires an explicit jobs root; the CLI's
//! environment/XDG root resolution is not applied to embedded callers, so a
//! consumer never accidentally shares a root it did not choose.
//! [`EmbeddedClient::with_supervisor_exe`] overrides the supervisor executable
//! for tests and unusual packaging where the delegating binary is not the
//! current executable.
//!
//! ## Lifecycle guarantees
//!
//! * `run` returns only after the delegated supervisor has acknowledged startup
//!   within [`crate::run::SUPERVISOR_ACK_TIMEOUT`], so a `running` result means
//!   supervision really claimed the job.
//! * A launch that never produced validated supervision persists terminal
//!   `failed` with no intermediate `running` transition, launches no workload,
//!   and emits no completion notification.
//! * After a successful `run`, the job outlives the launching process. Another
//!   client bound to the same root observes and controls it.
//!
//! ## API shape
//!
//! The API is synchronous, matching the underlying jobstore. Every operation
//! returns [`JobError`] on failure, whose [`JobError::kind`] and
//! [`JobError::is_retryable`] let callers branch on missing jobs, ambiguous
//! IDs, invalid input, invalid state, launch failure, and I/O or internal
//! failure without parsing message text.

use std::path::{Path, PathBuf};

use crate::gc::AutoGcConfig;
use crate::jobstore::{AmbiguousJobId, InvalidJobState, JobIdCollisionExhausted, JobNotFound};
use crate::run::{StdinRequired, StdinTooLarge, SupervisorLaunchFailed};
use crate::tag::InvalidTag;

// The embedded surface returns the same domain types the rest of the crate
// persists and serializes; re-exporting them keeps consumers off `crate::schema`
// internals while avoiding a parallel set of near-identical structs.
pub use crate::compress::CompressionMode;
pub use crate::run::StdinSource;
pub use crate::schema::{JobStatus, JobSummary, KillData, ListData, RunData, StatusData, TailData};

/// Reserved `argv[1]` marker that identifies a supervisor invocation.
///
/// Private dispatch token; not a stable end-user CLI surface.
pub const SUPERVISOR_MARKER: &str = "_supervise";

/// Outcome of [`delegate_supervisor_startup`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorDelegation {
    /// The invocation is not a reserved supervisor invocation. The consumer's
    /// arguments were not consumed or modified; continue normal startup.
    NotSupervisor,
    /// The invocation was claimed and supervision ran to a terminal state.
    /// The consumer's normal command handling MUST NOT run.
    Supervised,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Stable classification of an embedded operation failure.
///
/// Consumers branch on this instead of matching message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JobErrorKind {
    /// No job matched the supplied ID or prefix.
    JobNotFound,
    /// A job ID prefix matched more than one job.
    AmbiguousJobId,
    /// The request itself was rejected (bad tag, bad signal, missing/oversized stdin, ...).
    InvalidInput,
    /// The job exists but is in a state that forbids the operation.
    InvalidState,
    /// The supervisor executable could not be launched or never acknowledged startup.
    LaunchFailed,
    /// Storage or other I/O failure.
    Io,
    /// Anything else.
    Internal,
}

impl JobErrorKind {
    /// Stable string form, matching the CLI JSON error codes where one exists.
    pub fn as_str(self) -> &'static str {
        match self {
            JobErrorKind::JobNotFound => "job_not_found",
            JobErrorKind::AmbiguousJobId => "ambiguous_job_id",
            JobErrorKind::InvalidInput => "invalid_input",
            JobErrorKind::InvalidState => "invalid_state",
            JobErrorKind::LaunchFailed => "launch_failed",
            JobErrorKind::Io => "io_error",
            JobErrorKind::Internal => "internal_error",
        }
    }
}

/// Error returned by every [`EmbeddedClient`] operation.
#[derive(Debug)]
pub struct JobError {
    kind: JobErrorKind,
    retryable: bool,
    message: String,
    candidates: Vec<String>,
    source: anyhow::Error,
}

impl JobError {
    /// Stable machine-readable classification.
    pub fn kind(&self) -> JobErrorKind {
        self.kind
    }

    /// Whether retrying the identical request could plausibly succeed.
    pub fn is_retryable(&self) -> bool {
        self.retryable
    }

    /// Human-readable detail. Never parse this; branch on [`JobError::kind`].
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Candidate job IDs for [`JobErrorKind::AmbiguousJobId`]; empty otherwise.
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    /// The underlying error, preserved so CLI/MCP/HTTP adapters can keep their
    /// existing downcast-based code mapping.
    pub fn into_source(self) -> anyhow::Error {
        self.source
    }

    /// Classify an `anyhow::Error` produced by the shared command implementations.
    pub fn from_anyhow(err: anyhow::Error) -> Self {
        let message = format!("{err:#}");
        let (kind, candidates) = if let Some(amb) = err.downcast_ref::<AmbiguousJobId>() {
            (JobErrorKind::AmbiguousJobId, amb.candidates.clone())
        } else if err.downcast_ref::<JobNotFound>().is_some() {
            (JobErrorKind::JobNotFound, Vec::new())
        } else if err.downcast_ref::<InvalidJobState>().is_some() {
            (JobErrorKind::InvalidState, Vec::new())
        } else if err.downcast_ref::<SupervisorLaunchFailed>().is_some() {
            (JobErrorKind::LaunchFailed, Vec::new())
        } else if err.downcast_ref::<InvalidTag>().is_some()
            || err.downcast_ref::<StdinRequired>().is_some()
            || err.downcast_ref::<StdinTooLarge>().is_some()
            || err.downcast_ref::<crate::config::ConfigError>().is_some()
        {
            (JobErrorKind::InvalidInput, Vec::new())
        } else if err.downcast_ref::<JobIdCollisionExhausted>().is_some()
            || err.downcast_ref::<std::io::Error>().is_some()
        {
            (JobErrorKind::Io, Vec::new())
        } else {
            (JobErrorKind::Internal, Vec::new())
        };

        // Only ID exhaustion is worth an automatic retry: every other category
        // reflects a decision the caller has to change.
        let retryable = matches!(kind, JobErrorKind::Io)
            && err.downcast_ref::<JobIdCollisionExhausted>().is_some();

        JobError {
            kind,
            retryable,
            message,
            candidates,
            source: err,
        }
    }

    fn invalid_input(message: impl Into<String>) -> Self {
        let message = message.into();
        JobError {
            kind: JobErrorKind::InvalidInput,
            retryable: false,
            message: message.clone(),
            candidates: Vec::new(),
            source: anyhow::anyhow!(message),
        }
    }
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for JobError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.source()
    }
}

impl From<anyhow::Error> for JobError {
    fn from(err: anyhow::Error) -> Self {
        JobError::from_anyhow(err)
    }
}

/// Convenience result alias for embedded operations.
pub type JobResult<T> = std::result::Result<T, JobError>;

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

/// Typed `run` request.
///
/// Field defaults mirror the CLI defaults, including the bounded inline
/// observation (`wait = true`, `until_seconds = 10`).
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Command to run. One element is a shell command string; more than one is argv.
    pub command: Vec<String>,
    /// Observe bounded initial output before returning (CLI default: true).
    pub wait: bool,
    /// Inline observation budget in seconds.
    pub until_seconds: u64,
    /// Observe until the job reaches a terminal state, ignoring `until_seconds`.
    pub forever: bool,
    /// Maximum bytes taken from the head of each stream for the response.
    pub max_bytes: u64,
    /// Workload timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds between SIGTERM and SIGKILL on timeout; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the workload.
    pub cwd: Option<String>,
    /// `KEY=VALUE` pairs applied to the workload environment.
    pub env_vars: Vec<String>,
    /// Env files applied in order before `env_vars`.
    pub env_files: Vec<String>,
    /// Inherit the launching process environment (default: true).
    pub inherit_env: bool,
    /// Keys whose values are masked in persisted metadata and results.
    pub mask: Vec<String>,
    /// Stdin source materialized into the job directory.
    pub stdin: Option<StdinSource>,
    /// Maximum materialized stdin size in bytes.
    pub stdin_max_bytes: u64,
    /// Tags persisted for recovery/filtering.
    pub tags: Vec<String>,
    /// Override for the combined `full.log` path.
    pub log: Option<String>,
    /// Interval in milliseconds for `state.json` liveness refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// Shell command executed on completion.
    pub notify_command: Option<String>,
    /// NDJSON file that receives completion events.
    pub notify_file: Option<String>,
    /// Output-match pattern.
    pub output_pattern: Option<String>,
    /// Output-match type: `contains` or `regex`.
    pub output_match_type: Option<String>,
    /// Output-match stream: `stdout`, `stderr`, or `either`.
    pub output_stream: Option<String>,
    /// Output-match command sink.
    pub output_command: Option<String>,
    /// Output-match NDJSON file sink.
    pub output_file: Option<String>,
    /// Shell wrapper argv used to execute command strings.
    pub shell_wrapper: Vec<String>,
    /// Disable best-effort auto-GC for this launch.
    pub no_auto_gc: bool,
    /// Auto-GC settings when auto-GC is enabled.
    pub auto_gc_config: AutoGcConfig,
    /// Output compression mode for the returned excerpts.
    pub compression_mode: CompressionMode,
}

impl Default for RunRequest {
    fn default() -> Self {
        let cli_defaults = crate::run::RunOpts::default();
        RunRequest {
            command: Vec::new(),
            wait: cli_defaults.wait,
            until_seconds: cli_defaults.until_seconds,
            forever: cli_defaults.forever,
            max_bytes: cli_defaults.max_bytes,
            timeout_ms: cli_defaults.timeout_ms,
            kill_after_ms: cli_defaults.kill_after_ms,
            cwd: None,
            env_vars: Vec::new(),
            env_files: Vec::new(),
            inherit_env: cli_defaults.inherit_env,
            mask: Vec::new(),
            stdin: None,
            stdin_max_bytes: cli_defaults.stdin_max_bytes,
            tags: Vec::new(),
            log: None,
            progress_every_ms: cli_defaults.progress_every_ms,
            notify_command: None,
            notify_file: None,
            output_pattern: None,
            output_match_type: None,
            output_stream: None,
            output_command: None,
            output_file: None,
            shell_wrapper: cli_defaults.shell_wrapper,
            no_auto_gc: cli_defaults.no_auto_gc,
            auto_gc_config: cli_defaults.auto_gc_config,
            compression_mode: cli_defaults.compression_mode,
        }
    }
}

impl RunRequest {
    /// A run request with CLI-equivalent defaults for `command`.
    pub fn new(command: Vec<String>) -> Self {
        RunRequest {
            command,
            ..RunRequest::default()
        }
    }

    /// Launch without the bounded inline observation.
    ///
    /// The fixed supervisor startup acknowledgement still applies: this skips
    /// workload observation, not the launch-integrity check.
    pub fn no_wait(mut self) -> Self {
        self.wait = false;
        self
    }

    /// Attach recovery tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Typed `tail` request.
#[derive(Debug, Clone)]
pub struct TailRequest {
    /// Job ID or unambiguous prefix.
    pub job_id: String,
    /// Lines taken from the end of each log.
    pub tail_lines: u64,
    /// Maximum bytes read from the end of each log.
    pub max_bytes: u64,
    /// Output compression mode for the returned excerpts.
    pub compression_mode: CompressionMode,
}

impl TailRequest {
    /// A tail request with CLI-equivalent defaults.
    pub fn new(job_id: impl Into<String>) -> Self {
        let cli_defaults = crate::tail::TailOpts::default();
        TailRequest {
            job_id: job_id.into(),
            tail_lines: cli_defaults.tail_lines,
            max_bytes: cli_defaults.max_bytes,
            compression_mode: cli_defaults.compression_mode,
        }
    }
}

/// Typed `list` request.
#[derive(Debug, Clone, Default)]
pub struct ListRequest {
    /// Maximum jobs returned; 0 = unlimited.
    pub limit: u64,
    /// Optional state filter (`running`, `exited`, `killed`, `failed`, `unknown`).
    pub state: Option<String>,
    /// Only jobs created from this directory. Ignored when `all` is set.
    pub cwd: Option<String>,
    /// Disable cwd filtering entirely.
    ///
    /// Embedded callers usually want this: the launching process's working
    /// directory is rarely the right recovery scope.
    pub all: bool,
    /// Tag patterns; all must match (logical AND). Supports `ns.*` prefixes.
    pub tags: Vec<String>,
}

impl ListRequest {
    /// List every job under the client's root, unfiltered by cwd.
    pub fn all() -> Self {
        ListRequest {
            all: true,
            ..ListRequest::default()
        }
    }

    /// Restrict to jobs matching every supplied tag pattern.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Cap the number of returned jobs.
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }
}

/// Typed `kill` request.
#[derive(Debug, Clone)]
pub struct KillRequest {
    /// Job ID or unambiguous prefix.
    pub job_id: String,
    /// `TERM`, `INT`, or `KILL` (case-insensitive). Unknown values map to `KILL`.
    pub signal: String,
    /// Return immediately instead of observing the post-signal terminal state.
    pub no_wait: bool,
}

impl KillRequest {
    /// A `TERM` kill request that observes the post-signal state.
    pub fn new(job_id: impl Into<String>) -> Self {
        KillRequest {
            job_id: job_id.into(),
            signal: "TERM".to_string(),
            no_wait: false,
        }
    }

    /// Use a different signal name.
    pub fn with_signal(mut self, signal: impl Into<String>) -> Self {
        self.signal = signal.into();
        self
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Typed managed-job client bound to one explicit jobs root.
///
/// Cheap to clone and safe to construct per operation. See the module docs for
/// the required startup sequence.
#[derive(Debug, Clone)]
pub struct EmbeddedClient {
    root: PathBuf,
    supervisor_exe: PathBuf,
}

impl EmbeddedClient {
    /// Create a client whose supervisor executable is the current executable.
    ///
    /// This is correct only when the current executable calls
    /// [`delegate_supervisor_startup`] before its own argument parsing. A
    /// consumer that forgets gets a [`JobErrorKind::LaunchFailed`] from `run`
    /// within [`crate::run::SUPERVISOR_ACK_TIMEOUT`] rather than a silently
    /// broken job.
    pub fn new(root: impl Into<PathBuf>) -> JobResult<Self> {
        let supervisor_exe = crate::run::default_supervisor_exe().map_err(JobError::from_anyhow)?;
        Ok(EmbeddedClient {
            root: root.into(),
            supervisor_exe,
        })
    }

    /// Create a client with an explicit trusted supervisor executable.
    ///
    /// For tests and packaging layouts where the delegating binary is not the
    /// current executable. The executable MUST install
    /// [`delegate_supervisor_startup`].
    pub fn with_supervisor_exe(
        root: impl Into<PathBuf>,
        supervisor_exe: impl Into<PathBuf>,
    ) -> Self {
        EmbeddedClient {
            root: root.into(),
            supervisor_exe: supervisor_exe.into(),
        }
    }

    /// The explicit jobs root this client operates on.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The executable re-executed for detached supervision.
    pub fn supervisor_exe(&self) -> &Path {
        &self.supervisor_exe
    }

    fn root_arg(&self) -> String {
        self.root.display().to_string()
    }

    /// Launch a managed job and observe its bounded initial output.
    ///
    /// Returns only after the detached supervisor acknowledged startup, so a
    /// non-`failed` result means supervision owns the job and will keep running
    /// after this process exits.
    pub fn run(&self, request: RunRequest) -> JobResult<RunData> {
        if request.command.is_empty() {
            return Err(JobError::invalid_input("no command specified for run"));
        }
        let root_arg = self.root_arg();
        crate::run::run_response(crate::run::RunOpts {
            command: request.command,
            root: Some(root_arg.as_str()),
            no_auto_gc: request.no_auto_gc,
            auto_gc_older_than: None,
            auto_gc_max_jobs: None,
            auto_gc_max_bytes: None,
            auto_gc_config: request.auto_gc_config,
            wait: request.wait,
            until_seconds: request.until_seconds,
            forever: request.forever,
            max_bytes: request.max_bytes,
            compression_mode: request.compression_mode,
            timeout_ms: request.timeout_ms,
            kill_after_ms: request.kill_after_ms,
            cwd: request.cwd.as_deref(),
            env_vars: request.env_vars,
            env_files: request.env_files,
            inherit_env: request.inherit_env,
            mask: request.mask,
            stdin: request.stdin,
            stdin_max_bytes: request.stdin_max_bytes,
            tags: request.tags,
            log: request.log.as_deref(),
            progress_every_ms: request.progress_every_ms,
            notify_command: request.notify_command,
            notify_file: request.notify_file,
            output_pattern: request.output_pattern,
            output_match_type: request.output_match_type,
            output_stream: request.output_stream,
            output_command: request.output_command,
            output_file: request.output_file,
            shell_wrapper: request.shell_wrapper,
            supervisor_exe: Some(self.supervisor_exe.clone()),
        })
        .map(|response| response.data)
        .map_err(JobError::from_anyhow)
    }

    /// Read a job's managed state.
    pub fn status(&self, job_id: &str) -> JobResult<StatusData> {
        let root_arg = self.root_arg();
        crate::status::status_response(crate::status::StatusOpts {
            job_id,
            root: Some(root_arg.as_str()),
        })
        .map(|response| response.data)
        .map_err(JobError::from_anyhow)
    }

    /// Read bounded log tails for a job.
    pub fn tail(&self, request: TailRequest) -> JobResult<TailData> {
        let root_arg = self.root_arg();
        crate::tail::tail_response(crate::tail::TailOpts {
            job_id: request.job_id.as_str(),
            root: Some(root_arg.as_str()),
            tail_lines: request.tail_lines,
            max_bytes: request.max_bytes,
            compression_mode: request.compression_mode,
        })
        .map(|response| response.data)
        .map_err(JobError::from_anyhow)
    }

    /// Enumerate jobs under this client's root.
    pub fn list(&self, request: ListRequest) -> JobResult<ListData> {
        let root_arg = self.root_arg();
        crate::list::list_data(crate::list::ListOpts {
            root: Some(root_arg.as_str()),
            limit: request.limit,
            state: request.state.as_deref(),
            cwd: request.cwd.as_deref(),
            all: request.all,
            tags: request.tags,
        })
        .map_err(JobError::from_anyhow)
    }

    /// Signal a job's process tree and observe the resulting terminal state.
    pub fn kill(&self, request: KillRequest) -> JobResult<KillData> {
        let root_arg = self.root_arg();
        crate::kill::execute_inner(crate::kill::KillOpts {
            job_id: request.job_id.as_str(),
            root: Some(root_arg.as_str()),
            signal: request.signal.as_str(),
            no_wait: request.no_wait,
        })
        .map_err(JobError::from_anyhow)
    }
}

// ---------------------------------------------------------------------------
// Supervisor startup delegation
// ---------------------------------------------------------------------------

/// Claim and run a reserved supervisor invocation, if this process is one.
///
/// Call this at the very top of `main`, before any consumer argument parsing.
/// Returns [`SupervisorDelegation::NotSupervisor`] without consuming or
/// modifying arguments when `argv[1]` is not exactly [`SUPERVISOR_MARKER`].
pub fn delegate_supervisor_startup() -> JobResult<SupervisorDelegation> {
    delegate_supervisor_startup_from(std::env::args_os())
}

/// Whether this process was started as a reserved supervisor invocation.
///
/// Useful for consumers that need to configure logging or skip expensive
/// startup work before calling [`delegate_supervisor_startup`].
pub fn is_supervisor_invocation() -> bool {
    std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == std::ffi::OsStr::new(SUPERVISOR_MARKER))
}

/// [`delegate_supervisor_startup`] over an explicit argument vector.
///
/// `args` is a full argv including `args[0]`.
pub fn delegate_supervisor_startup_from<I, S>(args: I) -> JobResult<SupervisorDelegation>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>,
{
    let argv: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    if argv.len() < 2 || argv[1] != std::ffi::OsStr::new(SUPERVISOR_MARKER) {
        return Ok(SupervisorDelegation::NotSupervisor);
    }

    // Claimed. From here on nothing may fall through to consumer handling.
    let parsed = SuperviseArgs::parse(&argv[2..]).map_err(JobError::invalid_input)?;
    parsed.supervise().map_err(JobError::from_anyhow)?;
    Ok(SupervisorDelegation::Supervised)
}

/// Strictly parsed reserved supervisor invocation.
///
/// The grammar is generated by [`crate::run::spawn_supervisor_process`] and is
/// private. Parsing is deliberately strict: unknown flags, missing values,
/// duplicated single-valued flags, conflicting flags, positional arguments
/// before `--`, and an empty workload are all rejected.
#[derive(Debug, Default)]
struct SuperviseArgs {
    job_id: Option<String>,
    supervise_root: Option<String>,
    full_log: Option<String>,
    timeout: Option<u64>,
    kill_after: Option<u64>,
    cwd: Option<String>,
    env_vars: Vec<String>,
    env_files: Vec<String>,
    no_inherit_env: bool,
    inherit_env: bool,
    stdin_file: Option<String>,
    progress_every: Option<u64>,
    notify_command: Option<String>,
    notify_file: Option<String>,
    shell_wrapper: Option<String>,
    shell_wrapper_resolved: Option<String>,
    command: Vec<String>,
}

impl SuperviseArgs {
    fn parse(args: &[std::ffi::OsString]) -> Result<Self, String> {
        let mut parsed = SuperviseArgs::default();
        let mut iter = args.iter();
        let mut saw_separator = false;

        while let Some(raw) = iter.next() {
            let arg = raw
                .to_str()
                .ok_or_else(|| "supervisor argument is not valid UTF-8".to_string())?;

            if arg == "--" {
                saw_separator = true;
                for value in iter.by_ref() {
                    let value = value
                        .to_str()
                        .ok_or_else(|| "workload argument is not valid UTF-8".to_string())?;
                    parsed.command.push(value.to_string());
                }
                break;
            }

            if !arg.starts_with("--") {
                return Err(format!(
                    "unexpected positional supervisor argument {arg:?} before `--`"
                ));
            }

            let mut next_value = |flag: &str| -> Result<String, String> {
                iter.next()
                    .ok_or_else(|| format!("supervisor flag {flag} is missing its value"))
                    .and_then(|v| {
                        v.to_str()
                            .map(str::to_string)
                            .ok_or_else(|| format!("value for {flag} is not valid UTF-8"))
                    })
            };

            match arg {
                "--job-id" => set_once(&mut parsed.job_id, next_value(arg)?, arg)?,
                "--supervise-root" => set_once(&mut parsed.supervise_root, next_value(arg)?, arg)?,
                "--full-log" => set_once(&mut parsed.full_log, next_value(arg)?, arg)?,
                "--timeout" => {
                    set_once(&mut parsed.timeout, parse_u64(&next_value(arg)?, arg)?, arg)?
                }
                "--kill-after" => set_once(
                    &mut parsed.kill_after,
                    parse_u64(&next_value(arg)?, arg)?,
                    arg,
                )?,
                "--cwd" => set_once(&mut parsed.cwd, next_value(arg)?, arg)?,
                "--env" => parsed.env_vars.push(next_value(arg)?),
                "--env-file" => parsed.env_files.push(next_value(arg)?),
                "--no-inherit-env" => set_flag_once(&mut parsed.no_inherit_env, arg)?,
                "--inherit-env" => set_flag_once(&mut parsed.inherit_env, arg)?,
                "--stdin-file" => set_once(&mut parsed.stdin_file, next_value(arg)?, arg)?,
                "--progress-every" => set_once(
                    &mut parsed.progress_every,
                    parse_u64(&next_value(arg)?, arg)?,
                    arg,
                )?,
                "--notify-command" => set_once(&mut parsed.notify_command, next_value(arg)?, arg)?,
                "--notify-file" => set_once(&mut parsed.notify_file, next_value(arg)?, arg)?,
                "--shell-wrapper" => set_once(&mut parsed.shell_wrapper, next_value(arg)?, arg)?,
                "--shell-wrapper-resolved" => {
                    set_once(&mut parsed.shell_wrapper_resolved, next_value(arg)?, arg)?
                }
                other => return Err(format!("unknown supervisor flag {other}")),
            }
        }

        if parsed.job_id.is_none() {
            return Err("supervisor invocation is missing --job-id".to_string());
        }
        if parsed.supervise_root.is_none() {
            return Err("supervisor invocation is missing --supervise-root".to_string());
        }
        if parsed.no_inherit_env && parsed.inherit_env {
            return Err("--inherit-env conflicts with --no-inherit-env".to_string());
        }
        if !saw_separator {
            return Err("supervisor invocation is missing the `--` workload separator".to_string());
        }
        if parsed.command.is_empty() {
            return Err("supervisor invocation has an empty workload".to_string());
        }

        Ok(parsed)
    }

    fn supervise(self) -> anyhow::Result<()> {
        let shell_wrapper = match self.shell_wrapper_resolved {
            Some(ref json) => serde_json::from_str::<Vec<String>>(json)
                .map_err(|e| anyhow::anyhow!("parse --shell-wrapper-resolved JSON: {e}"))?,
            None => crate::config::resolve_shell_wrapper(self.shell_wrapper.as_deref(), None)?,
        };
        let root = PathBuf::from(self.supervise_root.as_deref().unwrap_or_default());
        crate::run::supervise(crate::run::SuperviseOpts {
            job_id: self.job_id.as_deref().unwrap_or_default(),
            root: root.as_path(),
            command: &self.command,
            full_log: self.full_log.as_deref(),
            timeout_ms: self.timeout.unwrap_or(0).saturating_mul(1000),
            kill_after_ms: self.kill_after.unwrap_or(0).saturating_mul(1000),
            cwd: self.cwd.as_deref(),
            env_vars: self.env_vars.clone(),
            env_files: self.env_files.clone(),
            inherit_env: !self.no_inherit_env,
            stdin_file: self.stdin_file.clone(),
            progress_every_ms: self.progress_every.unwrap_or(0),
            notify_command: self.notify_command.clone(),
            notify_file: self.notify_file.clone(),
            shell_wrapper,
        })
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "supervisor flag {flag} was supplied more than once"
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn set_flag_once(slot: &mut bool, flag: &str) -> Result<(), String> {
    if *slot {
        return Err(format!(
            "supervisor flag {flag} was supplied more than once"
        ));
    }
    *slot = true;
    Ok(())
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|e| format!("supervisor flag {flag} expects an integer, got {value:?}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn argv(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn ordinary_consumer_invocation_is_not_claimed() {
        let result =
            delegate_supervisor_startup_from(argv(&["my-consumer", "serve", "--port", "8080"]))
                .expect("delegation must not fail for ordinary arguments");
        assert_eq!(result, SupervisorDelegation::NotSupervisor);
    }

    #[test]
    fn bare_invocation_is_not_claimed() {
        let result = delegate_supervisor_startup_from(argv(&["my-consumer"])).expect("no args");
        assert_eq!(result, SupervisorDelegation::NotSupervisor);
    }

    #[test]
    fn marker_is_only_claimed_at_argv1() {
        // A marker-looking value in a later position belongs to the consumer.
        let result =
            delegate_supervisor_startup_from(argv(&["my-consumer", "run", SUPERVISOR_MARKER]))
                .expect("later marker is consumer data");
        assert_eq!(result, SupervisorDelegation::NotSupervisor);
    }

    #[test]
    fn near_miss_marker_is_not_claimed() {
        for near in ["_supervise2", "supervise", "--_supervise", "_Supervise"] {
            let result = delegate_supervisor_startup_from(argv(&["my-consumer", near]))
                .unwrap_or_else(|e| panic!("near miss {near} must not be claimed: {e}"));
            assert_eq!(
                result,
                SupervisorDelegation::NotSupervisor,
                "near miss {near} must not be claimed"
            );
        }
    }

    #[test]
    fn claimed_invocation_with_missing_job_id_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--supervise-root",
            "/tmp/root",
            "--",
            "echo hi",
        ]))
        .expect_err("missing --job-id must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
        assert!(err.message().contains("--job-id"), "{}", err.message());
    }

    #[test]
    fn claimed_invocation_with_duplicate_flag_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--job-id",
            "def",
            "--supervise-root",
            "/tmp/root",
            "--",
            "echo hi",
        ]))
        .expect_err("duplicate --job-id must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
        assert!(
            err.message().contains("more than once"),
            "{}",
            err.message()
        );
    }

    #[test]
    fn claimed_invocation_with_unknown_flag_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
            "--totally-unknown",
            "--",
            "echo hi",
        ]))
        .expect_err("unknown flag must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
        assert!(err.message().contains("unknown"), "{}", err.message());
    }

    #[test]
    fn claimed_invocation_with_positional_before_separator_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
            "surprise",
            "--",
            "echo hi",
        ]))
        .expect_err("trailing positional must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
        assert!(err.message().contains("positional"), "{}", err.message());
    }

    #[test]
    fn claimed_invocation_without_separator_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
        ]))
        .expect_err("missing separator must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
    }

    #[test]
    fn claimed_invocation_with_empty_workload_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
            "--",
        ]))
        .expect_err("empty workload must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
    }

    #[test]
    fn claimed_invocation_with_malformed_number_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
            "--timeout",
            "soon",
            "--",
            "echo hi",
        ]))
        .expect_err("malformed --timeout must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
    }

    #[test]
    fn claimed_invocation_with_missing_value_fails_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
        ]))
        .expect_err("missing value must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
    }

    #[test]
    fn conflicting_env_inheritance_flags_fail_closed() {
        let err = delegate_supervisor_startup_from(argv(&[
            "my-consumer",
            SUPERVISOR_MARKER,
            "--job-id",
            "abc",
            "--supervise-root",
            "/tmp/root",
            "--inherit-env",
            "--no-inherit-env",
            "--",
            "echo hi",
        ]))
        .expect_err("conflicting env flags must fail closed");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
    }

    #[test]
    fn generated_argument_grammar_parses() {
        let parsed = SuperviseArgs::parse(&argv(&[
            "--job-id",
            "abc123",
            "--supervise-root",
            "/tmp/root",
            "--full-log",
            "/tmp/root/abc123/full.log",
            "--timeout",
            "5",
            "--kill-after",
            "2",
            "--cwd",
            "/tmp",
            "--env-file",
            "/tmp/a.env",
            "--env",
            "K=V",
            "--env",
            "K2=V2",
            "--no-inherit-env",
            "--stdin-file",
            "stdin.bin",
            "--progress-every",
            "1",
            "--notify-command",
            "true",
            "--notify-file",
            "/tmp/events.ndjson",
            "--shell-wrapper-resolved",
            "[\"sh\",\"-lc\"]",
            "--",
            "echo hello",
        ]))
        .expect("generated grammar must parse");

        assert_eq!(parsed.job_id.as_deref(), Some("abc123"));
        assert_eq!(parsed.supervise_root.as_deref(), Some("/tmp/root"));
        assert_eq!(parsed.timeout, Some(5));
        assert_eq!(parsed.kill_after, Some(2));
        assert_eq!(
            parsed.env_vars,
            vec!["K=V".to_string(), "K2=V2".to_string()]
        );
        assert_eq!(parsed.env_files, vec!["/tmp/a.env".to_string()]);
        assert!(parsed.no_inherit_env);
        assert_eq!(parsed.progress_every, Some(1));
        assert_eq!(parsed.command, vec!["echo hello".to_string()]);
    }

    #[test]
    fn workload_flags_after_separator_are_not_parsed_as_supervisor_flags() {
        let parsed = SuperviseArgs::parse(&argv(&[
            "--job-id",
            "abc123",
            "--supervise-root",
            "/tmp/root",
            "--",
            "my-tool",
            "--job-id",
            "not-a-supervisor-flag",
        ]))
        .expect("workload argv is opaque");
        assert_eq!(parsed.job_id.as_deref(), Some("abc123"));
        assert_eq!(
            parsed.command,
            vec![
                "my-tool".to_string(),
                "--job-id".to_string(),
                "not-a-supervisor-flag".to_string()
            ]
        );
    }

    #[test]
    fn error_kinds_expose_stable_codes() {
        assert_eq!(JobErrorKind::JobNotFound.as_str(), "job_not_found");
        assert_eq!(JobErrorKind::AmbiguousJobId.as_str(), "ambiguous_job_id");
        assert_eq!(JobErrorKind::InvalidState.as_str(), "invalid_state");
        assert_eq!(JobErrorKind::LaunchFailed.as_str(), "launch_failed");
        assert_eq!(JobErrorKind::Io.as_str(), "io_error");
        assert_eq!(JobErrorKind::Internal.as_str(), "internal_error");
    }

    #[test]
    fn anyhow_errors_classify_without_message_parsing() {
        let not_found = JobError::from_anyhow(anyhow::Error::new(JobNotFound("abc".into())));
        assert_eq!(not_found.kind(), JobErrorKind::JobNotFound);
        assert!(!not_found.is_retryable());

        let ambiguous = JobError::from_anyhow(anyhow::Error::new(AmbiguousJobId {
            prefix: "ab".into(),
            candidates: vec!["abc".into(), "abd".into()],
        }));
        assert_eq!(ambiguous.kind(), JobErrorKind::AmbiguousJobId);
        assert_eq!(
            ambiguous.candidates(),
            ["abc".to_string(), "abd".to_string()]
        );

        let invalid_state =
            JobError::from_anyhow(anyhow::Error::new(InvalidJobState("created".into())));
        assert_eq!(invalid_state.kind(), JobErrorKind::InvalidState);

        let launch = JobError::from_anyhow(anyhow::Error::new(SupervisorLaunchFailed(
            "no delegation".into(),
        )));
        assert_eq!(launch.kind(), JobErrorKind::LaunchFailed);

        let exhausted =
            JobError::from_anyhow(anyhow::Error::new(JobIdCollisionExhausted { attempts: 16 }));
        assert_eq!(exhausted.kind(), JobErrorKind::Io);
        assert!(exhausted.is_retryable());

        let other = JobError::from_anyhow(anyhow::anyhow!("something else"));
        assert_eq!(other.kind(), JobErrorKind::Internal);
    }

    #[test]
    fn run_request_defaults_match_cli_defaults() {
        let cli = crate::run::RunOpts::default();
        let request = RunRequest::new(vec!["echo hi".to_string()]);
        assert_eq!(request.wait, cli.wait);
        assert_eq!(request.until_seconds, cli.until_seconds);
        assert_eq!(request.max_bytes, cli.max_bytes);
        assert_eq!(request.stdin_max_bytes, cli.stdin_max_bytes);
        assert_eq!(request.inherit_env, cli.inherit_env);
        assert_eq!(request.shell_wrapper, cli.shell_wrapper);
        assert!(!request.no_wait().wait);
    }

    #[test]
    fn empty_command_is_rejected_before_any_job_directory_is_created() {
        let temp = tempfile::tempdir().expect("tempdir");
        let client = EmbeddedClient::with_supervisor_exe(temp.path(), "/nonexistent/supervisor");
        let err = client
            .run(RunRequest::new(vec![]))
            .expect_err("empty command must be rejected");
        assert_eq!(err.kind(), JobErrorKind::InvalidInput);
        assert_eq!(
            std::fs::read_dir(temp.path()).into_iter().flatten().count(),
            0,
            "no job directory may be created for an invalid request"
        );
    }
}
