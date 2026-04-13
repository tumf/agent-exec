//! Implementation of the `create` sub-command.
//!
//! `create` persists a full job definition without launching the supervisor or
//! child process.  The job is left in `created` state so that `start` can
//! launch it later.

use anyhow::{Context, Result};
use tracing::info;

use crate::jobstore::{JobDir, generate_job_id, resolve_root};
use crate::run::{
    mask_env_vars, materialize_stdin_for_job, pre_create_log_files, resolve_effective_cwd,
    validate_stdin_source,
};
use crate::schema::{CreateData, JobMeta, JobMetaJob, Response};
use crate::tag::dedup_tags;

/// Options for the `create` sub-command.
///
/// # Definition-time option alignment rule
///
/// Every definition-time option accepted here MUST also be accepted by `run` (and vice versa),
/// since both commands write the same persisted job definition to `meta.json`. When adding a
/// new persisted metadata field, wire it through both `create` and `run` unless the spec
/// explicitly documents it as launch-only (e.g. snapshot timing, tail sizing, --wait).
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
    /// Optional stdin source definition persisted and materialized for start.
    pub stdin: Option<crate::run::StdinSource>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// Shell command string for command notification sink.
    pub notify_command: Option<String>,
    /// File path for NDJSON notification sink.
    pub notify_file: Option<String>,
    /// Resolved shell wrapper argv (e.g. ["sh", "-lc"]).
    pub shell_wrapper: Vec<String>,
    /// User-defined tags for this job (deduplicated preserving first-seen order).
    pub tags: Vec<String>,
    /// Pattern to match against output lines (output-match notification).
    pub output_pattern: Option<String>,
    /// Match type for output-match: "contains" or "regex".
    pub output_match_type: Option<String>,
    /// Stream selector: "stdout", "stderr", or "either".
    pub output_stream: Option<String>,
    /// Shell command string for output-match command sink.
    pub output_command: Option<String>,
    /// File path for output-match NDJSON file sink.
    pub output_file: Option<String>,
}

/// Execute `create`: persist job definition and return JSON.
pub fn execute(opts: CreateOpts) -> Result<()> {
    if opts.command.is_empty() {
        anyhow::bail!("no command specified for create");
    }

    let root = resolve_root(opts.root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create jobs root {}", root.display()))?;

    let job_id = generate_job_id(&root)?;
    let created_at = crate::run::now_rfc3339_pub();

    let env_keys: Vec<String> = opts
        .env_vars
        .iter()
        .map(|kv| kv.split('=').next().unwrap_or(kv.as_str()).to_string())
        .collect();

    let masked_env_vars = mask_env_vars(&opts.env_vars, &opts.mask);

    let effective_cwd = resolve_effective_cwd(opts.cwd);

    // Build output-match config from definition-time options (same logic as `notify set`).
    let on_output_match = crate::notify::build_output_match_config(
        opts.output_pattern,
        opts.output_match_type,
        opts.output_stream,
        opts.output_command,
        opts.output_file,
        None,
    );

    let notification =
        if opts.notify_command.is_some() || opts.notify_file.is_some() || on_output_match.is_some()
        {
            Some(crate::schema::NotificationConfig {
                notify_command: opts.notify_command.clone(),
                notify_file: opts.notify_file.clone(),
                on_output_match,
            })
        } else {
            None
        };

    // Validate and deduplicate tags (preserving first-seen order).
    let tags = dedup_tags(opts.tags)?;

    let stdin_source = opts.stdin.clone();
    validate_stdin_source(stdin_source.as_ref())?;

    let meta = JobMeta {
        job: JobMetaJob { id: job_id.clone() },
        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
        command: opts.command.clone(),
        created_at: created_at.clone(),
        root: root.display().to_string(),
        env_keys,
        env_vars: masked_env_vars,
        // Persist actual (unmasked) env vars for runtime use by `start`.
        // --mask only affects display/metadata views; the real values are needed
        // so `start` can apply them to the child process environment.
        env_vars_runtime: opts.env_vars.clone(),
        mask: opts.mask.clone(),
        cwd: Some(effective_cwd),
        notification,
        tags,
        // Execution-definition fields persisted for `start`.
        inherit_env: opts.inherit_env,
        env_files: opts.env_files.clone(),
        timeout_ms: opts.timeout_ms,
        kill_after_ms: opts.kill_after_ms,
        progress_every_ms: opts.progress_every_ms,
        shell_wrapper: Some(opts.shell_wrapper.clone()),
        stdin_file: None,
    };

    let job_dir = JobDir::create(&root, &job_id, &meta)?;
    let stdin_file = materialize_stdin_for_job(&job_dir, stdin_source.as_ref())?;
    if stdin_file.is_some() {
        let mut meta_with_stdin = meta.clone();
        meta_with_stdin.stdin_file = stdin_file;
        job_dir.write_meta_atomic(&meta_with_stdin)?;
    }
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
