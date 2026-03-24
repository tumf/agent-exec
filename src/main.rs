//! agent-exec v0.1 — entry point
//!
//! All stdout is JSON only. Tracing logs go to stderr.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use agent_exec::jobstore::{InvalidJobState, JobNotFound};
use agent_exec::schema::ErrorResponse;
use agent_exec::skills::UnknownSourceScheme;
use agent_exec::tag::InvalidTag;

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

#[derive(Debug, Parser)]
#[command(name = "agent-exec")]
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

        /// Timeout in milliseconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Milliseconds after SIGTERM to send SIGKILL; 0 = immediate SIGKILL on timeout.
        #[arg(long, default_value = "0")]
        kill_after: u64,

        /// Working directory for the command.
        #[arg(long)]
        cwd: Option<String>,

        /// Set environment variable KEY=VALUE (persisted as durable config; may be repeated).
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from a file (persisted as path reference; may be repeated).
        #[arg(long = "env-file", value_name = "FILE")]
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

        /// Interval (ms) at which state.json.updated_at is refreshed; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Shell command string to run on job completion.
        #[arg(long, value_name = "COMMAND")]
        notify_command: Option<String>,

        /// File path that receives one NDJSON `job.finished` event per completed job.
        #[arg(long, value_name = "PATH")]
        notify_file: Option<String>,

        /// Path to a config.toml file to load (overrides XDG default).
        #[arg(long, value_name = "PATH")]
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
        #[arg(long, value_name = "PATH")]
        output_file: Option<String>,

        /// Command and arguments to run when `start` is called.
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Start a previously created job. Returns JSON with type="start".
    Start {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Wait N ms before returning (0 = return immediately, default = 10000ms).
        #[arg(long, default_value = "10000")]
        snapshot_after: u64,

        /// Number of tail lines to include in snapshot.
        #[arg(long, default_value = "50")]
        tail_lines: u64,

        /// Maximum bytes for tail.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Wait for the job to reach a terminal state before returning.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        wait: bool,

        /// Poll interval in milliseconds while waiting for a terminal state.
        #[arg(long, default_value = "200")]
        wait_poll_ms: u64,

        /// Job ID of a previously created job.
        job_id: String,
    },

    /// Run a command as a background job and return JSON immediately.
    Run {
        /// Wait N ms before returning (0 = return immediately, default = 10000ms).
        #[arg(long, default_value = "10000")]
        snapshot_after: u64,

        /// Number of tail lines to include in snapshot.
        #[arg(long, default_value = "50")]
        tail_lines: u64,

        /// Maximum bytes for tail.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Timeout in milliseconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Milliseconds after SIGTERM to send SIGKILL; 0 = immediate SIGKILL on timeout.
        #[arg(long, default_value = "0")]
        kill_after: u64,

        /// Working directory for the command.
        #[arg(long)]
        cwd: Option<String>,

        /// Set environment variable KEY=VALUE (may be repeated).
        #[arg(long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from a file (may be repeated, applied in order).
        #[arg(long = "env-file", value_name = "FILE")]
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

        /// Assign a tag to this job (may be repeated; duplicates are deduplicated).
        #[arg(long = "tag", value_name = "TAG", value_parser = parse_stored_tag)]
        tags: Vec<String>,

        /// Override full.log path.
        #[arg(long)]
        log: Option<String>,

        /// Interval (ms) at which state.json.updated_at is refreshed; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Wait for the job to reach a terminal state before returning.
        /// When set, the response includes exit_code, finished_at, and final_snapshot.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        wait: bool,

        /// Poll interval in milliseconds while waiting for a terminal state.
        #[arg(long, default_value = "200")]
        wait_poll_ms: u64,

        /// Shell command string to run on job completion; executed via the configured shell
        /// wrapper. Event JSON is sent to stdin.
        /// Also sets AGENT_EXEC_EVENT_PATH, AGENT_EXEC_JOB_ID, and AGENT_EXEC_EVENT_TYPE.
        #[arg(long, value_name = "COMMAND")]
        notify_command: Option<String>,

        /// File path that receives one NDJSON `job.finished` event per completed job.
        #[arg(long, value_name = "PATH")]
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
        #[arg(long = "output-file", value_name = "PATH")]
        output_file: Option<String>,

        /// Path to a config.toml file to load (overrides XDG default).
        #[arg(long, value_name = "PATH")]
        config: Option<String>,

        /// Shell wrapper program and flags used to execute command strings
        /// (e.g. "bash -lc"). Overrides the config file and built-in default.
        #[arg(long, value_name = "PROGRAM AND FLAGS")]
        shell_wrapper: Option<String>,

        /// Command and arguments to run.
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Get status of a job.
    Status {
        /// Job ID.
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
        job_id: String,
    },

    /// Wait for a job to finish.
    Wait {
        /// Poll interval in milliseconds.
        #[arg(long, default_value = "200")]
        poll_ms: u64,

        /// Timeout in milliseconds (0 = indefinite).
        #[arg(long, default_value = "0")]
        timeout_ms: u64,

        /// Job ID.
        job_id: String,
    },

    /// Send a signal to a job.
    Kill {
        /// Signal: TERM | INT | KILL (default: TERM).
        #[arg(long, default_value = "TERM")]
        signal: String,

        /// Job ID.
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
        #[arg(required_unless_present = "all", conflicts_with = "all")]
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
        #[arg(long, default_value = "0")]
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

    /// Install agent skills into .agents/skills/.
    #[command(name = "install-skills")]
    InstallSkills {
        /// Source specification: "self" (built-in) or "local:<path>".
        #[arg(long, default_value = "self")]
        source: String,

        /// Install into ~/.agents/ instead of ./.agents/.
        #[arg(long, default_value = "false", action = clap::ArgAction::SetTrue)]
        global: bool,
    },

    /// Manage job notification configuration.
    Notify {
        #[command(subcommand)]
        subcommand: NotifySubcommand,
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

        /// Timeout in milliseconds; 0 = no timeout.
        #[arg(long, default_value = "0")]
        timeout: u64,

        /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
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

        /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

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
    let cli = Cli::parse();

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
        // "unknown_source_scheme" is not retryable: the source scheme is invalid.
        // "invalid_tag" is not retryable: the tag value is malformed.
        // "internal_error" is not retryable by default; a transient I/O error
        // would need its own code+retryable=true if we ever surface it.
        if e.downcast_ref::<JobNotFound>().is_some() {
            ErrorResponse::new("job_not_found", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<UnknownSourceScheme>().is_some() {
            ErrorResponse::new("unknown_source_scheme", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<InvalidTag>().is_some() {
            ErrorResponse::new("invalid_tag", format!("{e:#}"), false).print();
        } else if e.downcast_ref::<InvalidJobState>().is_some() {
            ErrorResponse::new("invalid_state", format!("{e:#}"), false).print();
        } else {
            ErrorResponse::new("internal_error", format!("{e:#}"), false).print();
        }
        std::process::exit(1);
    }
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
            agent_exec::create::execute(agent_exec::create::CreateOpts {
                command,
                root: root.as_deref(),
                timeout_ms: timeout,
                kill_after_ms: kill_after,
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                mask,
                progress_every_ms: progress_every,
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
            snapshot_after,
            tail_lines,
            max_bytes,
            wait,
            wait_poll_ms,
            job_id,
        } => {
            agent_exec::start::execute(agent_exec::start::StartOpts {
                job_id: &job_id,
                root: root.as_deref(),
                snapshot_after,
                tail_lines,
                max_bytes,
                wait,
                wait_poll_ms,
            })?;
        }

        Command::Run {
            snapshot_after,
            tail_lines,
            max_bytes,
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
            wait,
            wait_poll_ms,
            notify_command,
            notify_file,
            output_pattern,
            output_match_type,
            output_stream,
            output_command,
            output_file,
            config,
            shell_wrapper,
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
            agent_exec::run::execute(agent_exec::run::RunOpts {
                command,
                root: root.as_deref(),
                snapshot_after,
                tail_lines,
                max_bytes,
                timeout_ms: timeout,
                kill_after_ms: kill_after,
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                mask,
                tags,
                log: log.as_deref(),
                progress_every_ms: progress_every,
                wait,
                wait_poll_ms,
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
            poll_ms,
            timeout_ms,
            job_id,
        } => {
            agent_exec::wait::execute(agent_exec::wait::WaitOpts {
                job_id: &job_id,
                root: root.as_deref(),
                poll_ms,
                timeout_ms,
            })?;
        }

        Command::Kill { signal, job_id } => {
            agent_exec::kill::execute(agent_exec::kill::KillOpts {
                job_id: &job_id,
                root: root.as_deref(),
                signal: &signal,
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

        Command::Schema => {
            agent_exec::schema_cmd::execute(agent_exec::schema_cmd::SchemaOpts)?;
        }

        Command::InstallSkills { source, global } => {
            agent_exec::install_skills::execute(agent_exec::install_skills::InstallSkillsOpts {
                source: &source,
                global,
            })?;
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
                timeout_ms: timeout,
                kill_after_ms: kill_after,
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                progress_every_ms: progress_every,
                notify_command,
                notify_file,
                shell_wrapper: resolved_wrapper,
            })?;
        }
    }
    Ok(())
}
