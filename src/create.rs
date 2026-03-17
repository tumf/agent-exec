//! Implementation of the `create` sub-command.
//!
//! `create` persists a full job definition without launching the supervisor or
//! child process.  The job is left in `created` state so that `start` can
//! launch it later.

use anyhow::{Context, Result};
use tracing::info;
use ulid::Ulid;

use crate::jobstore::{JobDir, resolve_root};
use crate::run::{mask_env_vars, pre_create_log_files, resolve_effective_cwd};
use crate::schema::{
    CreateData, JobMeta, JobMetaJob, Response,
};

/// Options for the `create` sub-command.
#[derive(Debug)]
pub struct CreateOpts<'a> {
    /// Command and arguments to execute when `start` is called.
    pub command: Vec<String>,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the command.
    pub cwd: Option<&'a str>,
    /// Environment variables as KEY=VALUE strings (persisted as durable config).
    pub env_vars: Vec<String>,
    /// Paths to env files (persisted as file-path references, read at start time).
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment at start time (default: true).
    pub inherit_env: bool,
    /// Keys to mask in JSON output (values replaced with "***").
    pub mask: Vec<String>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// Shell command string for command notification sink.
    pub notify_command: Option<String>,
    /// File path for NDJSON notification sink.
    pub notify_file: Option<String>,
    /// Resolved shell wrapper argv (e.g. ["sh", "-lc"]).
    pub shell_wrapper: Vec<String>,
}

/// Execute `create`: persist job definition and return JSON.
pub fn execute(opts: CreateOpts) -> Result<()> {
    if opts.command.is_empty() {
        anyhow::bail!("no command specified for create");
    }

    let root = resolve_root(opts.root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create jobs root {}", root.display()))?;

    let job_id = Ulid::new().to_string();
    let created_at = crate::run::now_rfc3339_pub();

    let env_keys: Vec<String> = opts
        .env_vars
        .iter()
        .map(|kv| kv.split('=').next().unwrap_or(kv.as_str()).to_string())
        .collect();

    let masked_env_vars = mask_env_vars(&opts.env_vars, &opts.mask);

    let effective_cwd = resolve_effective_cwd(opts.cwd);

    let notification = if opts.notify_command.is_some() || opts.notify_file.is_some() {
        Some(crate::schema::NotificationConfig {
            notify_command: opts.notify_command.clone(),
            notify_file: opts.notify_file.clone(),
        })
    } else {
        None
    };

    let meta = JobMeta {
        job: JobMetaJob { id: job_id.clone() },
        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
        command: opts.command.clone(),
        created_at: created_at.clone(),
        root: root.display().to_string(),
        env_keys,
        env_vars: masked_env_vars,
        mask: opts.mask.clone(),
        cwd: Some(effective_cwd),
        notification,
        // Execution-definition fields persisted for `start`.
        inherit_env: opts.inherit_env,
        env_files: opts.env_files.clone(),
        timeout_ms: opts.timeout_ms,
        kill_after_ms: opts.kill_after_ms,
        progress_every_ms: opts.progress_every_ms,
        shell_wrapper: Some(opts.shell_wrapper.clone()),
    };

    let job_dir = JobDir::create(&root, &job_id, &meta)?;
    info!(job_id = %job_id, "created job directory (created state)");

    // Pre-create empty log files.
    pre_create_log_files(&job_dir)?;

    // Write state.json with `created` status — no process spawned.
    job_dir.init_state_created()?;

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    Response::new(
        "create",
        CreateData {
            job_id,
            state: "created".to_string(),
            stdout_log_path,
            stderr_log_path,
        },
    )
    .print();

    Ok(())
}
