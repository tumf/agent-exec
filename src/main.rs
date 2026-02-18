//! agent-exec v0.1 — entry point
//!
//! All stdout is JSON only. Tracing logs go to stderr.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use agent_shell::jobstore::JobNotFound;
use agent_shell::schema::ErrorResponse;

#[derive(Debug, Parser)]
#[command(name = "agent-exec")]
#[command(about = "Non-interactive agent job runner", long_about = None)]
struct Cli {
    /// Increase log verbosity (-v, -vv); logs go to stderr.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a command as a background job and return JSON immediately.
    Run {
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

        /// Override full.log path.
        #[arg(long)]
        log: Option<String>,

        /// Interval (ms) at which state.json.updated_at is refreshed; 0 = disabled.
        #[arg(long, default_value = "0")]
        progress_every: u64,

        /// Command and arguments to run.
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Get status of a job.
    Status {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Job ID.
        job_id: String,
    },

    /// Get stdout/stderr tail of a job.
    Tail {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

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
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

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
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Signal: TERM | INT | KILL (default: TERM).
        #[arg(long, default_value = "TERM")]
        signal: String,

        /// Job ID.
        job_id: String,
    },

    /// List all jobs under the root directory.
    List {
        /// Override jobs root directory.
        #[arg(long)]
        root: Option<String>,

        /// Maximum number of jobs to return (0 = no limit).
        #[arg(long, default_value = "0")]
        limit: u64,
    },

    /// [Internal] Supervise a child process — not for direct use.
    #[command(name = "_supervise", hide = true)]
    Supervise {
        #[arg(long)]
        job_id: String,

        #[arg(long)]
        root: String,

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

        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

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
        // "internal_error" is not retryable by default; a transient I/O error
        // would need its own code+retryable=true if we ever surface it.
        if e.downcast_ref::<JobNotFound>().is_some() {
            ErrorResponse::new("job_not_found", format!("{e:#}"), false).print();
        } else {
            ErrorResponse::new("internal_error", format!("{e:#}"), false).print();
        }
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Run {
            root,
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
            log,
            progress_every,
            command,
        } => {
            // --inherit-env and --no-inherit-env are mutually exclusive (enforced by clap).
            // If neither is specified, default is to inherit (inherit_env=true).
            // If --no-inherit-env is set, inherit_env=false.
            let should_inherit = !no_inherit_env;
            agent_shell::run::execute(agent_shell::run::RunOpts {
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
                log: log.as_deref(),
                progress_every_ms: progress_every,
            })?;
        }

        Command::Status { root, job_id } => {
            agent_shell::status::execute(agent_shell::status::StatusOpts {
                job_id: &job_id,
                root: root.as_deref(),
            })?;
        }

        Command::Tail {
            root,
            tail_lines,
            max_bytes,
            job_id,
        } => {
            agent_shell::tail::execute(agent_shell::tail::TailOpts {
                job_id: &job_id,
                root: root.as_deref(),
                tail_lines,
                max_bytes,
            })?;
        }

        Command::Wait {
            root,
            poll_ms,
            timeout_ms,
            job_id,
        } => {
            agent_shell::wait::execute(agent_shell::wait::WaitOpts {
                job_id: &job_id,
                root: root.as_deref(),
                poll_ms,
                timeout_ms,
            })?;
        }

        Command::Kill {
            root,
            signal,
            job_id,
        } => {
            agent_shell::kill::execute(agent_shell::kill::KillOpts {
                job_id: &job_id,
                root: root.as_deref(),
                signal: &signal,
            })?;
        }

        Command::List { root, limit } => {
            agent_shell::list::execute(agent_shell::list::ListOpts {
                root: root.as_deref(),
                limit,
            })?;
        }

        Command::Supervise {
            job_id,
            root,
            full_log,
            timeout,
            kill_after,
            cwd,
            env_vars,
            env_files,
            no_inherit_env,
            inherit_env: _inherit_env,
            progress_every,
            command,
        } => {
            let should_inherit = !no_inherit_env;
            agent_shell::run::supervise(agent_shell::run::SuperviseOpts {
                job_id: &job_id,
                root: std::path::Path::new(&root),
                command: &command,
                full_log: full_log.as_deref(),
                timeout_ms: timeout,
                kill_after_ms: kill_after,
                cwd: cwd.as_deref(),
                env_vars,
                env_files,
                inherit_env: should_inherit,
                progress_every_ms: progress_every,
            })?;
        }
    }
    Ok(())
}
