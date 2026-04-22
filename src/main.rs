//! agent-exec v0.1 — entry point
//!
//! All stdout is JSON only. Tracing logs go to stderr.

use anyhow::{Context, Result};
use clap::builder::ValueHint;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{CompleteEnv, Shell, engine::ArgValueCompleter};
use std::ffi::OsString;

use tracing_subscriber::EnvFilter;

use agent_exec::jobstore::{AmbiguousJobId, InvalidJobState, JobIdCollisionExhausted, JobNotFound};
use agent_exec::schema::ErrorResponse;
use agent_exec::tag::InvalidTag;

/// Shell variants supported by the `completions` subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
}

impl From<CompletionShell> for Shell {
    fn from(s: CompletionShell) -> Shell {
        match s {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
        }
    }
}

impl CompletionShell {
    fn env_name(self) -> &'static str {
        match self {
            CompletionShell::Bash => "bash",
            CompletionShell::Zsh => "zsh",
            CompletionShell::Fish => "fish",
            CompletionShell::PowerShell => "powershell",
        }
    }
}

/// Clap value parser: validate a stored tag (used by `run` and `tag set`).
fn parse_stored_tag(s: &str) -> Result<String, String> {
    agent_exec::tag::validate_stored_tag(s)
        .map(|()| s.to_string())
        .map_err(|e| e.to_string())
}

/// Clap value parser: validate a list filter pattern (used by `list`).
fn parse_filter_pattern(s: &str) -> Result<String, String> {
    agent_exec::tag::validate_filter_pattern(s)
        .map(|()| s.to_string())
        .map_err(|e| e.to_string())
}

/// Custom value parser for `--signal`: exposes common signal names as completion
/// candidates while still accepting any arbitrary signal string at runtime.
#[derive(Clone, Debug)]
struct SignalValueParser;

impl clap::builder::TypedValueParser for SignalValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::error::Error> {
        Ok(value.to_string_lossy().to_string())
    }

    fn possible_values(
        &self,
    ) -> Option<Box<dyn Iterator<Item = clap::builder::PossibleValue> + '_>> {
        Some(Box::new(
            ["TERM", "INT", "KILL", "HUP", "USR1", "USR2"]
                .iter()
                .map(|s| clap::builder::PossibleValue::new(*s)),
        ))
    }
}

#[derive(Debug, Parser)]
#[command(name = "agent-exec")]
#[command(version)]
#[command(about = "Non-interactive agent job runner", long_about = None)]
struct Cli {
    /// Increase log verbosity (-v, -vv); logs go to stderr.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Override jobs root directory (applies to all job-store subcommands).
    /// Precedence: --root > AGENT_EXEC_ROOT > $XDG_DATA_HOME/agent-exec/jobs > platform default.
    #[arg(long, global = true, value_name = "PATH")]
    root: Option<String>,

    /// Output responses as YAML instead of JSON (applies to all subcommands).
    #[arg(long, global = true, default_value = "false", action = clap::ArgAction::SetTrue)]
    yaml: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a job definition without starting it. Returns JSON with type="create".
    Create {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Timeout in seconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Seconds after SIGTERM to send SIGKILL; 0 = immediate SIGKILL on timeout.
        #[arg(long, default_value = "0")]
        kill_after: u64,

        /// Working directory for the command.
        #[arg(long, value_hint = ValueHint::DirPath)]
        cwd: Option<String>,

        /// Set environment variable KEY=VALUE (persisted as durable config; may be repeated).
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from a file (persisted as path reference; may be repeated).
        #[arg(long = "env-file", value_name = "FILE", value_hint = ValueHint::FilePath)]
        env_files: Vec<String>,

        /// Do not inherit the current process environment at start time.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "inherit_env")]
        no_inherit_env: bool,

        /// Inherit the current process environment at start time (default; conflicts with --no-inherit-env).
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "no_inherit_env")]
        inherit_env: bool,

        /// Mask secret values in JSON output (key name only; may be repeated).
        #[arg(long = "mask", value_name = "KEY")]
        mask: Vec<String>,

        /// Provide stdin content directly. Use `--stdin -` to read from caller stdin.
        #[arg(long, value_name = "VALUE", conflicts_with = "stdin_file")]
        stdin: Option<String>,

        /// Read stdin content from file and materialize it into the job directory.
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, conflicts_with = "stdin")]
        stdin_file: Option<String>,

        /// Maximum bytes allowed for materialized stdin.bin (default: 64 MiB).
        #[arg(long, value_name = "BYTES", default_value_t = agent_exec::run::DEFAULT_STDIN_MAX_BYTES)]
        stdin_max_bytes: u64,

        /// Interval (seconds) at which state.json.updated_at is refreshed; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Shell command string to run on job completion.
        #[arg(long, value_name = "COMMAND")]
        notify_command: Option<String>,

        /// File path that receives one NDJSON `job.finished` event per completed job.
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        notify_file: Option<String>,

        /// Path to a config.toml file to load (overrides XDG default).
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        config: Option<String>,

        /// Shell wrapper program and flags (e.g. "bash -lc"). Overrides config and built-in default.
        #[arg(long, value_name = "PROGRAM AND FLAGS")]
        shell_wrapper: Option<String>,

        /// Assign a tag to this job (may be repeated; duplicates are deduplicated).
        #[arg(long = "tag", value_name = "TAG", value_parser = parse_stored_tag)]
        tags: Vec<String>,

        /// Pattern to match against output lines (enables output-match notifications).
        #[arg(long, value_name = "PATTERN")]
        output_pattern: Option<String>,

        /// Match type for output-match: contains (default) or regex.
        #[arg(long, value_name = "TYPE", value_parser = ["contains", "regex"])]
        output_match_type: Option<String>,

        /// Stream to match: stdout, stderr, or either (default).
        #[arg(long, value_name = "STREAM", value_parser = ["stdout", "stderr", "either"])]
        output_stream: Option<String>,

        /// Shell command string to execute on output match.
        #[arg(long, value_name = "COMMAND")]
        output_command: Option<String>,

        /// File path that receives one NDJSON event per output match.
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        output_file: Option<String>,

        /// Command and arguments to run when `start` is called.
        #[arg(required = true, trailing_var_arg = true, value_hint = ValueHint::CommandWithArguments)]
        command: Vec<String>,
    },

    /// Start a previously created job. Returns JSON with type="start".
    Start {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Wait for inline output observation before returning.
        /// Bare `--wait` is treated as `true`; `--wait true|false` remains supported.
        #[arg(
            long,
            default_value_t = true,
            default_missing_value = "true",
            num_args = 0..=1,
            action = clap::ArgAction::Set
        )]
        wait: bool,

        /// Maximum wait time in seconds for inline observation.
        #[arg(long, default_value = "10", conflicts_with = "forever")]
        until: u64,

        /// Wait indefinitely for terminal state / observation budget.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "until")]
        forever: bool,

        /// Alias for `--wait false --until 0`.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        no_wait: bool,

        /// Maximum bytes to include from the head of each stream.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Job ID of a previously created job.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_created_jobs))]
        job_id: String,
    },

    /// Run a command as a background job and return JSON immediately.
    Run {
        /// Timeout in seconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Seconds after SIGTERM to send SIGKILL; 0 = immediate SIGKILL on timeout.
        #[arg(long, default_value = "0")]
        kill_after: u64,

        /// Working directory for the command.
        #[arg(long, value_hint = ValueHint::DirPath)]
        cwd: Option<String>,

        /// Set environment variable KEY=VALUE (may be repeated).
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from a file (may be repeated, applied in order).
        #[arg(long = "env-file", value_name = "FILE", value_hint = ValueHint::FilePath)]
        env_files: Vec<String>,

        /// Do not inherit the current process environment.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "inherit_env")]
        no_inherit_env: bool,

        /// Inherit the current process environment (default; conflicts with --no-inherit-env).
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "no_inherit_env")]
        inherit_env: bool,

        /// Mask secret values in JSON output (key name only, may be repeated).
        #[arg(long = "mask", value_name = "KEY")]
        mask: Vec<String>,

        /// Provide stdin content directly. Use `--stdin -` to read from caller stdin.
        #[arg(long, value_name = "VALUE", conflicts_with = "stdin_file")]
        stdin: Option<String>,

        /// Read stdin content from file and materialize it into the job directory.
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath, conflicts_with = "stdin")]
        stdin_file: Option<String>,

        /// Maximum bytes allowed for materialized stdin.bin (default: 64 MiB).
        #[arg(long, value_name = "BYTES", default_value_t = agent_exec::run::DEFAULT_STDIN_MAX_BYTES)]
        stdin_max_bytes: u64,

        /// Assign a tag to this job (may be repeated; duplicates are deduplicated).
        #[arg(long = "tag", value_name = "TAG", value_parser = parse_stored_tag)]
        tags: Vec<String>,

        /// Override full.log path.
        #[arg(long, value_hint = ValueHint::FilePath)]
        log: Option<String>,

        /// Interval (seconds) at which state.json.updated_at is refreshed; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Shell command string to run on job completion; executed via the configured shell
        /// wrapper. Event JSON is sent to stdin.
        /// Also sets AGENT_EXEC_EVENT_PATH, AGENT_EXEC_JOB_ID, and AGENT_EXEC_EVENT_TYPE.
        #[arg(long, value_name = "COMMAND")]
        notify_command: Option<String>,

        /// File path that receives one NDJSON `job.finished` event per completed job.
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        notify_file: Option<String>,

        /// Pattern to match against output lines (enables output-match notifications).
        #[arg(long, value_name = "PATTERN")]
        output_pattern: Option<String>,

        /// Match type for output-match: contains (default) or regex.
        #[arg(long, value_name = "TYPE", value_parser = ["contains", "regex"])]
        output_match_type: Option<String>,

        /// Stream to match: stdout, stderr, or either (default).
        #[arg(long, value_name = "STREAM", value_parser = ["stdout", "stderr", "either"])]
        output_stream: Option<String>,

        /// Shell command string to execute on output match.
        #[arg(long, value_name = "COMMAND")]
        output_command: Option<String>,

        /// File path that receives one NDJSON event per output match.
        #[arg(long = "output-file", value_name = "PATH", value_hint = ValueHint::FilePath)]
        output_file: Option<String>,

        /// Path to a config.toml file to load (overrides XDG default).
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        config: Option<String>,

        /// Shell wrapper program and flags used to execute command strings
        /// (e.g. "bash -lc"). Overrides the config file and built-in default.
        #[arg(long, value_name = "PROGRAM AND FLAGS")]
        shell_wrapper: Option<String>,

        /// Wait for inline output observation before returning.
        /// Bare `--wait` is treated as `true`; `--wait true|false` remains supported.
        #[arg(
            long,
            default_value_t = true,
            default_missing_value = "true",
            num_args = 0..=1,
            action = clap::ArgAction::Set
        )]
        wait: bool,

        /// Maximum wait time in seconds for inline observation.
        #[arg(long, default_value = "10", conflicts_with = "forever")]
        until: u64,

        /// Wait indefinitely for terminal state / observation budget.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "until")]
        forever: bool,

        /// Alias for `--wait false --until 0`.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        no_wait: bool,

        /// Maximum bytes to include from the head of each stream.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Command and arguments to run.
        #[arg(required = true, trailing_var_arg = true, value_hint = ValueHint::CommandWithArguments)]
        command: Vec<String>,
    },

    /// Get status of a job.
    Status {
        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_all_jobs))]
        job_id: String,
    },

    /// Get stdout/stderr tail of a job.
    Tail {
        /// Number of tail lines.
        #[arg(long, default_value = "50")]
        tail_lines: u64,

        /// Maximum bytes.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_all_jobs))]
        job_id: String,
    },

    /// Wait for a job to finish.
    Wait {
        /// Poll interval in seconds.
        #[arg(long = "poll", default_value = "1")]
        poll_seconds: u64,

        /// Maximum client-side wait deadline in seconds (default: 30).
        /// This controls how long `wait` polls and does not stop the underlying job;
        /// use `run --timeout` to enforce process runtime limits.
        #[arg(long, conflicts_with = "forever")]
        until: Option<u64>,

        /// Wait indefinitely until the job reaches a terminal state.
        #[arg(
            long,
            default_value = "false",
            action = clap::ArgAction::SetTrue,
            conflicts_with = "until"
        )]
        forever: bool,

        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_waitable_jobs))]
        job_id: String,
    },

    /// Send a signal to a job.
    Kill {
        /// Signal name to send (default: TERM).
        #[arg(long, default_value = "TERM", value_parser = SignalValueParser)]
        signal: String,

        /// Skip post-signal observation; return immediately with legacy shape.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        no_wait: bool,

        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_running_jobs))]
        job_id: String,
    },

    /// Delete one or all finished jobs.
    Delete {
        /// Delete all finished jobs whose persisted cwd matches the caller's current directory.
        /// Mutually exclusive with JOB_ID.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        all: bool,

        /// Report actions without performing any deletions.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        dry_run: bool,

        /// Job ID to delete. Mutually exclusive with --all.
        #[arg(required_unless_present = "all", conflicts_with = "all",
              add = ArgValueCompleter::new(agent_exec::completions::complete_terminal_jobs))]
        job_id: Option<String>,
    },

    /// Garbage collect old terminal job directories.
    Gc {
        /// Retention duration: jobs older than this are deleted (e.g. 30d, 24h, 7d).
        /// When omitted, defaults to 30d.
        #[arg(long, value_name = "DURATION")]
        older_than: Option<String>,

        /// Report candidates without deleting any directories.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        dry_run: bool,
    },

    /// Print the JSON Schema for all CLI response types.
    Schema,

    /// List all jobs under the root directory.
    List {
        /// Maximum number of jobs to return (0 = no limit).
        #[arg(long, default_value = "50")]
        limit: u64,

        /// Filter jobs by state: created|running|exited|killed|failed|unknown.
        #[arg(long, value_parser = ["created", "running", "exited", "killed", "failed", "unknown"])]
        state: Option<String>,

        /// Filter jobs by working directory (conflicts with --all).
        #[arg(long, conflicts_with = "all")]
        cwd: Option<String>,

        /// Show all jobs regardless of working directory (conflicts with --cwd).
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "cwd")]
        all: bool,

        /// Filter jobs by tag pattern (may be repeated; all patterns must match).
        /// Supports exact match (e.g. "aaa") and namespace prefix match (e.g. "hoge.*").
        #[arg(long = "tag", value_name = "PATTERN", value_parser = parse_filter_pattern)]
        tags: Vec<String>,
    },

    /// Manage job tags.
    Tag {
        #[command(subcommand)]
        subcommand: TagSubcommand,
    },

    /// Install the built-in agent-exec skill into .agents/skills/ or .claude/skills/.
    #[command(name = "install-skills")]
    InstallSkills {
        /// Install into the home directory instead of the current directory.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        global: bool,

        /// Use .claude/ root instead of .agents/.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        claude: bool,
    },

    /// Manage job notification configuration.
    Notify {
        #[command(subcommand)]
        subcommand: NotifySubcommand,
    },

    /// Generate shell completion registration scripts for bash, zsh, fish, or powershell.
    ///
    /// Source the generated script in your shell profile to enable tab-completion.
    /// The generated script calls back into `agent-exec` at completion time so
    /// dynamic job ID completion stays in sync with the current binary.
    /// Example (bash):
    ///   agent-exec completions bash >> ~/.bash_completion
    /// Example (zsh):
    ///   agent-exec completions zsh > ~/.zsh/completions/_agent-exec
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: CompletionShell,
    },

    /// Start an HTTP server exposing job operations as REST endpoints.
    Serve {
        /// Bind address (host:port). Defaults to 127.0.0.1:19263 (localhost only).
        /// Use 0.0.0.0:19263 to expose on all interfaces (requires --insecure).
        #[arg(long, default_value = "127.0.0.1:19263")]
        bind: String,

        /// Override port only (alternative to --bind when only the port should differ).
        #[arg(long, conflicts_with = "bind")]
        port: Option<u16>,

        /// Allow binding to non-loopback addresses (dangerous: exposes RCE endpoint).
        #[arg(long)]
        insecure: bool,

        /// Set allowed CORS origin. Wildcard '*' is rejected.
        #[arg(long)]
        allow_origin: Option<String>,
    },

    /// [Internal] Supervise a child process — not for direct use.
    #[command(name = "_supervise", hide = true)]
    Supervise {
        #[arg(long)]
        job_id: String,

        #[arg(long)]
        supervise_root: String,

        /// Override full.log path.
        #[arg(long)]
        full_log: Option<String>,

        /// Timeout in seconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Seconds after SIGTERM to send SIGKILL; 0 = immediate SIGKILL on timeout.
        #[arg(long, default_value = "0")]
        kill_after: u64,

        /// Working directory for the child process.
        #[arg(long)]
        cwd: Option<String>,

        /// Environment variable KEY=VALUE (may be repeated).
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from a file (may be repeated).
        #[arg(long = "env-file", value_name = "FILE")]
        env_files: Vec<String>,

        /// Do not inherit the current process environment.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "supervise_inherit_env")]
        no_inherit_env: bool,

        /// Inherit the current process environment (default; conflicts with --no-inherit-env).
        #[arg(long = "inherit-env", default_value = "false", action = clap::ArgAction::SetTrue, conflicts_with = "no_inherit_env", id = "supervise_inherit_env")]
        inherit_env: bool,

        /// Interval in seconds for state.json updated_at refresh; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Materialized stdin file path relative to the job directory (internal use).
        #[arg(long, value_name = "PATH", hide = true)]
        stdin_file: Option<String>,

        /// Shell command string to run on job completion; executed via the configured shell
        /// wrapper. Event JSON is sent to stdin.
        /// Also sets AGENT_EXEC_EVENT_PATH, AGENT_EXEC_JOB_ID, and AGENT_EXEC_EVENT_TYPE.
        #[arg(long, value_name = "COMMAND")]
        notify_command: Option<String>,

        /// File path that receives one NDJSON `job.finished` event per completed job.
        #[arg(long, value_name = "PATH")]
        notify_file: Option<String>,

        /// Shell wrapper override as a string (for direct user invocation; not used by `run`).
        #[arg(long, value_name = "PROGRAM AND FLAGS")]
        shell_wrapper: Option<String>,

        /// Pre-resolved shell wrapper argv as a JSON array (set by `run`, not by users).
        /// Takes precedence over --shell-wrapper when present.
        #[arg(long, value_name = "JSON", hide = true)]
        shell_wrapper_resolved: Option<String>,

        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum TagSubcommand {
    /// Replace all tags on an existing job.
    Set {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_all_jobs))]
        job_id: String,

        /// Tag to assign (may be repeated; replaces all existing tags).
        #[arg(long = "tag", value_name = "TAG", required = false, value_parser = parse_stored_tag)]
        tags: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum NotifySubcommand {
    /// Update the persisted notification configuration for an existing job.
    Set {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Job ID.
        #[arg(add = ArgValueCompleter::new(agent_exec::completions::complete_all_jobs))]
        job_id: String,

        /// Shell command string to execute on job completion.
        /// Replaces any previously configured notify_command; notify_file is preserved.
        #[arg(long, value_name = "COMMAND")]
        command: Option<String>,

        /// Pattern to match against output lines (enables output-match notifications).
        #[arg(long, value_name = "PATTERN")]
        output_pattern: Option<String>,

        /// Match type for output-match: contains (default) or regex.
        #[arg(long, value_name = "TYPE", value_parser = ["contains", "regex"])]
        output_match_type: Option<String>,

        /// Stream to match: stdout, stderr, or either (default).
        #[arg(long, value_name = "STREAM", value_parser = ["stdout", "stderr", "either"])]
        output_stream: Option<String>,

        /// Shell command string to execute on output match.
        #[arg(long, value_name = "COMMAND")]
        output_command: Option<String>,

        /// File path that receives one NDJSON event per output match.
        #[arg(long, value_name = "PATH")]
        output_file: Option<String>,
    },
}

fn main() {
    // Handle dynamic completion requests (invoked by the shell with COMPLETE=<shell>).
    // This must run before argument normalization and clap parsing so completion candidates
    // are returned without any JSON output or tracing initialisation.
    CompleteEnv::with_factory(Cli::command).complete();

    let normalized_args = normalize_wait_flags(std::env::args_os());
    let cli = Cli::parse_from(normalized_args);

    // Set output format before any subcommand runs (including error paths).
    agent_exec::schema::set_yaml_output(cli.yaml);

    let default_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    // Logs always go to stderr so stdout remains JSON-only.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();

    let result = run(cli);
    if let Err(e) = result {
        // Distinguish "job not found" from generic internal errors.
        // "job_not_found" is not retryable: the job does not exist.
        // "invalid_tag" is not retryable: the tag value is malformed.
        // "internal_error" is not retryable by default; a transient I/O error
        // would need its own code+retryable=true if we ever surface it.
        if let Some(amb) = e.downcast_ref::<AmbiguousJobId>() {
            let truncated = amb.candidates.len() > 20;
            let candidates: Vec<&str> =
                amb.candidates.iter().take(20).map(|s| s.as_str()).collect();
            ErrorResponse::new("ambiguous_job_id", format!("{e:#}"), false)
                .with_details(serde_json::json!({
                    "candidates": candidates,
                    "truncated": truncated,
                }))
                .print();
        } else if e.downcast_ref::<JobNotFound>().is_some() {
            ErrorResponse::new("job_not_found", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<InvalidTag>().is_some() {
            ErrorResponse::new("invalid_tag", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<InvalidJobState>().is_some() {
            ErrorResponse::new("invalid_state", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<JobIdCollisionExhausted>().is_some() {
            ErrorResponse::new("io_error", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<agent_exec::run::StdinRequired>().is_some() {
            ErrorResponse::new("stdin_required", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<agent_exec::run::StdinTooLarge>().is_some() {
            ErrorResponse::new("stdin_too_large", format!("{e:#}"), false).print();
        } else {
            ErrorResponse::new("internal_error", format!("{e:#}"), false).print();
        }
        std::process::exit(1);
    }
}

fn normalize_wait_flags<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut normalized = Vec::new();
    let mut iter = args.into_iter().peekable();
    let mut wait_alias_enabled = false;
    let mut wait_alias_phase_ended = false;

    while let Some(arg) = iter.next() {
        let arg_text = arg.to_string_lossy();

        if arg_text == "--" {
            normalized.push(arg);
            normalized.extend(iter);
            break;
        }

        if arg_text == "run" || arg_text == "start" {
            wait_alias_enabled = true;
            wait_alias_phase_ended = false;
            normalized.push(arg);
            continue;
        }

        if wait_alias_enabled && !wait_alias_phase_ended && arg_text == "--wait" {
            let should_insert_true = match iter.peek() {
                Some(next) if next.to_string_lossy() == "--" => true,
                Some(next) if next.to_string_lossy().starts_with('-') => true,
                Some(next) => {
                    let value = next.to_string_lossy();
                    !(value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false"))
                }
                None => true,
            };

            normalized.push(arg);
            if should_insert_true {
                normalized.push(OsString::from("true"));
            }
            continue;
        }

        // For `run` / `start`, the first non-option token begins positional parsing
        // (`COMMAND...` / `JOB_ID`). Do not rewrite any later tokens, which belong to
        // the child command argv for `run`.
        if wait_alias_enabled
            && !wait_alias_phase_ended
            && !arg_text.starts_with('-')
            && !arg_text.is_empty()
        {
            wait_alias_phase_ended = true;
        }

        normalized.push(arg);
    }

    normalized
}

fn run(cli: Cli) -> Result<()> {
    let root = cli.root;
    match cli.command {
        Command::Create {
            root,
            timeout,
            kill_after,
            cwd,
            env_vars,
            env_files,
            no_inherit_env,
            inherit_env: _inherit_env,
            mask,
            stdin,
            stdin_file,
            stdin_max_bytes,
            progress_every,
            notify_command,
            notify_file,
            config,
            shell_wrapper,
            tags,
            output_pattern,
            output_match_type,
            output_stream,
            output_command,
            output_file,
            command,
        } => {
            let should_inherit = !no_inherit_env;
            let resolved_wrapper = agent_exec::config::resolve_shell_wrapper(
                shell_wrapper.as_deref(),
                config.as_deref(),
            )?;
            let resolved_stdin = agent_exec::run::resolve_stdin_source(stdin, stdin_file);
            agent_exec::create::execute(agent_exec::create::CreateOpts {
                command,
                root: root.as_deref(),
                timeout_ms: timeout.saturating_mul(1000),
                kill_after_ms: kill_after.saturating_mul(1000),
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                mask,
                stdin: resolved_stdin,
                stdin_max_bytes,
                progress_every_ms: progress_every.saturating_mul(1000),
                notify_command,
                notify_file,
                shell_wrapper: resolved_wrapper,
                tags,
                output_pattern,
                output_match_type,
                output_stream,
                output_command,
                output_file,
            })?;
        }

        Command::Start {
            root,
            wait,
            until,
            forever,
            no_wait,
            max_bytes,
            job_id,
        } => {
            let effective_wait = if no_wait { false } else { wait };
            let effective_until_seconds = if no_wait { 0 } else { until };
            let effective_forever = if no_wait { false } else { forever };
            agent_exec::start::execute(agent_exec::start::StartOpts {
                job_id: &job_id,
                root: root.as_deref(),
                wait: effective_wait,
                until_seconds: effective_until_seconds,
                forever: effective_forever,
                max_bytes,
            })?;
        }

        Command::Run {
            timeout,
            kill_after,
            cwd,
            env_vars,
            env_files,
            no_inherit_env,
            inherit_env: _inherit_env,
            mask,
            tags,
            log,
            progress_every,
            notify_command,
            notify_file,
            output_pattern,
            output_match_type,
            output_stream,
            output_command,
            output_file,
            stdin,
            stdin_file,
            stdin_max_bytes,
            config,
            shell_wrapper,
            wait,
            until,
            forever,
            no_wait,
            max_bytes,
            command,
        } => {
            // --inherit-env and --no-inherit-env are mutually exclusive (enforced by clap).
            // If neither is specified, default is to inherit (inherit_env=true).
            // If --no-inherit-env is set, inherit_env=false.
            let should_inherit = !no_inherit_env;
            // Resolve the shell wrapper using CLI override, config file, or defaults.
            let resolved_wrapper = agent_exec::config::resolve_shell_wrapper(
                shell_wrapper.as_deref(),
                config.as_deref(),
            )?;
            let resolved_stdin = agent_exec::run::resolve_stdin_source(stdin, stdin_file);
            let effective_wait = if no_wait { false } else { wait };
            let effective_until_seconds = if no_wait { 0 } else { until };
            let effective_forever = if no_wait { false } else { forever };
            agent_exec::run::execute(agent_exec::run::RunOpts {
                command,
                root: root.as_deref(),
                wait: effective_wait,
                until_seconds: effective_until_seconds,
                forever: effective_forever,
                max_bytes,
                timeout_ms: timeout.saturating_mul(1000),
                kill_after_ms: kill_after.saturating_mul(1000),
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                mask,
                stdin: resolved_stdin,
                stdin_max_bytes,
                tags,
                log: log.as_deref(),
                progress_every_ms: progress_every.saturating_mul(1000),
                notify_command,
                notify_file,
                output_pattern,
                output_match_type,
                output_stream,
                output_command,
                output_file,
                shell_wrapper: resolved_wrapper,
            })?;
        }

        Command::Status { job_id } => {
            agent_exec::status::execute(agent_exec::status::StatusOpts {
                job_id: &job_id,
                root: root.as_deref(),
            })?;
        }

        Command::Tail {
            tail_lines,
            max_bytes,
            job_id,
        } => {
            agent_exec::tail::execute(agent_exec::tail::TailOpts {
                job_id: &job_id,
                root: root.as_deref(),
                tail_lines,
                max_bytes,
            })?;
        }

        Command::Wait {
            poll_seconds,
            until,
            forever,
            job_id,
        } => {
            agent_exec::wait::execute(agent_exec::wait::WaitOpts {
                job_id: &job_id,
                root: root.as_deref(),
                poll_seconds,
                until_seconds: until.unwrap_or(30),
                forever,
            })?;
        }

        Command::Kill {
            signal,
            no_wait,
            job_id,
        } => {
            agent_exec::kill::execute(agent_exec::kill::KillOpts {
                job_id: &job_id,
                root: root.as_deref(),
                signal: &signal,
                no_wait,
            })?;
        }

        Command::Delete {
            all,
            dry_run,
            job_id,
        } => {
            agent_exec::delete::execute(agent_exec::delete::DeleteOpts {
                root: root.as_deref(),
                job_id: job_id.as_deref(),
                all,
                dry_run,
            })?;
        }

        Command::Gc {
            older_than,
            dry_run,
        } => {
            agent_exec::gc::execute(agent_exec::gc::GcOpts {
                root: root.as_deref(),
                older_than: older_than.as_deref(),
                dry_run,
            })?;
        }

        Command::Serve {
            bind,
            port,
            insecure,
            allow_origin,
        } => {
            let effective_bind = if let Some(p) = port {
                format!("127.0.0.1:{p}")
            } else {
                bind
            };
            agent_exec::serve::execute(agent_exec::serve::ServeOpts {
                bind: effective_bind,
                root: root.clone(),
                insecure,
                allow_origin,
            })?;
        }

        Command::Schema => {
            agent_exec::schema_cmd::execute(agent_exec::schema_cmd::SchemaOpts)?;
        }

        Command::Completions { shell } => {
            let completer = std::env::current_exe()
                .context("resolve current executable for shell completions")?;
            let current_dir = std::env::current_dir().ok();

            // Reuse CompleteEnv's registration-script path so generated shell code
            // calls back into this binary and preserves dynamic ArgValueCompleter hooks.
            unsafe {
                std::env::set_var("COMPLETE", shell.env_name());
            }
            let completed = CompleteEnv::with_factory(Cli::command)
                .try_complete([completer.into_os_string()], current_dir.as_deref())
                .context("generate shell completion registration")?;
            anyhow::ensure!(completed, "completion registration was not generated");
        }

        Command::InstallSkills { global, claude } => {
            agent_exec::install_skills::execute(agent_exec::install_skills::InstallSkillsOpts {
                global,
                claude,
            })?
        }

        Command::List {
            limit,
            state,
            cwd,
            all,
            tags,
        } => {
            agent_exec::list::execute(agent_exec::list::ListOpts {
                root: root.as_deref(),
                limit,
                state: state.as_deref(),
                cwd: cwd.as_deref(),
                all,
                tags,
            })?;
        }

        Command::Tag {
            subcommand: TagSubcommand::Set { root, job_id, tags },
        } => {
            agent_exec::tag::execute(agent_exec::tag::TagOpts {
                root: root.as_deref(),
                job_id: &job_id,
                tags,
            })?;
        }

        Command::Notify {
            subcommand:
                NotifySubcommand::Set {
                    root,
                    job_id,
                    command,
                    output_pattern,
                    output_match_type,
                    output_stream,
                    output_command,
                    output_file,
                },
        } => {
            agent_exec::notify::set(agent_exec::notify::NotifySetOpts {
                job_id: &job_id,
                root: root.as_deref(),
                command,
                output_pattern,
                output_match_type,
                output_stream,
                output_command,
                output_file,
            })?;
        }

        Command::Supervise {
            job_id,
            supervise_root,
            full_log,
            timeout,
            kill_after,
            cwd,
            env_vars,
            env_files,
            no_inherit_env,
            inherit_env: _inherit_env,
            progress_every,
            stdin_file,
            notify_command,
            notify_file,
            shell_wrapper,
            shell_wrapper_resolved,
            command,
        } => {
            let should_inherit = !no_inherit_env;
            // Use the pre-resolved JSON wrapper from `run` if present (no join/split round-trip).
            // Fall back to resolving from the string override or defaults.
            let resolved_wrapper = if let Some(json) = shell_wrapper_resolved {
                serde_json::from_str::<Vec<String>>(&json)
                    .context("parse --shell-wrapper-resolved JSON")?
            } else {
                agent_exec::config::resolve_shell_wrapper(shell_wrapper.as_deref(), None)?
            };
            agent_exec::run::supervise(agent_exec::run::SuperviseOpts {
                job_id: &job_id,
                root: std::path::Path::new(&supervise_root),
                command: &command,
                full_log: full_log.as_deref(),
                timeout_ms: timeout.saturating_mul(1000),
                kill_after_ms: kill_after.saturating_mul(1000),
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                stdin_file,
                progress_every_ms: progress_every,
                notify_command,
                notify_file,
                shell_wrapper: resolved_wrapper,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn list_default_limit_is_50() {
        let cli = Cli::parse_from(["agent-exec", "list"]);
        match cli.command {
            Command::List { limit, .. } => assert_eq!(limit, 50),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_explicit_zero_means_no_limit() {
        let cli = Cli::parse_from(["agent-exec", "list", "--limit", "0"]);
        match cli.command {
            Command::List { limit, .. } => assert_eq!(limit, 0),
            other => panic!("expected List, got {other:?}"),
        }
    }
}
