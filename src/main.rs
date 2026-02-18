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

        /// Wait N ms before returning (0 = return immediately).
        #[arg(long, default_value = "0")]
        snapshot_after: u64,

        /// Number of tail lines to include in snapshot.
        #[arg(long, default_value = "50")]
        tail_lines: u64,

        /// Maximum bytes for tail.
        #[arg(long, default_value = "65536")]
        max_bytes: u64,

        /// Additional environment variables in KEY=VALUE format.
        /// Only the key names are stored in meta.json; values are not persisted.
        #[arg(long = "env", value_name = "KEY=VALUE", action = clap::ArgAction::Append)]
        env_vars: Vec<String>,

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

    /// [Internal] Supervise a child process — not for direct use.
    #[command(name = "_supervise", hide = true)]
    Supervise {
        #[arg(long)]
        job_id: String,

        #[arg(long)]
        root: String,

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
        if e.downcast_ref::<JobNotFound>().is_some() {
            ErrorResponse::new("job_not_found", format!("{e:#}")).print();
        } else {
            ErrorResponse::new("internal_error", format!("{e:#}")).print();
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
            env_vars,
            command,
        } => {
            agent_shell::run::execute(agent_shell::run::RunOpts {
                command,
                root: root.as_deref(),
                snapshot_after,
                tail_lines,
                max_bytes,
                env_vars,
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

        Command::Supervise {
            job_id,
            root,
            command,
        } => {
            agent_shell::run::supervise(&job_id, std::path::Path::new(&root), &command)?;
        }
    }
    Ok(())
}
