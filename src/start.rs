//! Implementation of the `start` sub-command.
//!
//! `start` launches a previously `create`d job.  It reads the persisted
//! execution definition from `meta.json`, validates that the job is in
//! `created` state, and then spawns the supervisor process.

use anyhow::Result;
use tracing::info;

use crate::jobstore::{InvalidJobState, JobDir, resolve_root};
use crate::run::{
    SpawnSupervisorParams, mask_env_vars, observe_inline_output, spawn_supervisor_process,
};
use crate::schema::{JobStatus, Response, RunData};

/// Options for the `start` sub-command.
#[derive(Debug)]
pub struct StartOpts<'a> {
    /// Job ID of a previously created job.
    pub job_id: &'a str,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Wait for inline output observation before returning.
    pub wait: bool,
    /// Maximum wait duration in seconds for inline observation.
    pub until_seconds: u64,
    /// Wait indefinitely for terminal state / observation budget.
    pub forever: bool,
    /// Maximum bytes to include from the head of each stream.
    pub max_bytes: u64,
}

/// Execute `start`: launch a created job and return JSON.
pub fn execute(opts: StartOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let meta = job_dir.read_meta()?;
    let state = job_dir.read_state()?;

    // Only jobs in `created` state can be started.
    if *state.status() != JobStatus::Created {
        return Err(anyhow::Error::new(InvalidJobState(format!(
            "job {} is in '{}' state; only 'created' jobs can be started",
            opts.job_id,
            state.status().as_str()
        ))));
    }

    info!(job_id = %opts.job_id, "starting created job");

    // Determine full.log path.
    let full_log_path = job_dir.full_log_path().display().to_string();

    // Resolve shell wrapper: use persisted value from meta, or re-resolve from config.
    let shell_wrapper = if let Some(ref w) = meta.shell_wrapper {
        w.clone()
    } else {
        crate::config::default_shell_wrapper()
    };

    // Use the persisted runtime env vars (unmasked) for the supervisor call.
    // env_vars_runtime stores the actual KEY=VALUE pairs written by `create`; this
    // ensures that `--mask KEY` only redacts the display/metadata view while the real
    // value is still applied to the child process environment at start time.
    // env_files are re-read here (deferred loading) so file contents reflect the
    // current state of the files at start time, not at create time.

    let (supervisor_pid, started_at) = spawn_supervisor_process(
        &job_dir,
        SpawnSupervisorParams {
            job_id: job_dir.job_id.clone(),
            root: root.clone(),
            full_log_path: full_log_path.clone(),
            timeout_ms: meta.timeout_ms,
            kill_after_ms: meta.kill_after_ms,
            cwd: meta.cwd.clone(),
            env_vars: meta.env_vars_runtime.clone(),
            env_files: meta.env_files.clone(),
            inherit_env: meta.inherit_env,
            stdin_file: meta.stdin_file.clone(),
            progress_every_ms: meta.progress_every_ms,
            notify_command: meta
                .notification
                .as_ref()
                .and_then(|n| n.notify_command.clone()),
            notify_file: meta
                .notification
                .as_ref()
                .and_then(|n| n.notify_file.clone()),
            shell_wrapper,
            command: meta.command.clone(),
        },
    )?;

    info!(job_id = %opts.job_id, supervisor_pid, started_at = %started_at, "job started");

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    // The response uses the masked env_vars (display view), not the runtime values.
    let masked_env_vars = mask_env_vars(&meta.env_vars_runtime, &meta.mask);
    let observation = observe_inline_output(
        &job_dir,
        opts.wait,
        opts.until_seconds,
        opts.forever,
        opts.max_bytes,
    )?;

    Response::new(
        "start",
        RunData {
            job_id: job_dir.job_id.clone(),
            state: observation.state,
            tags: meta.tags.clone(),
            env_vars: masked_env_vars,
            stdout_log_path,
            stderr_log_path,
            elapsed_ms: 0,
            waited_ms: observation.waited_ms,
            stdout: observation.stdout,
            stderr: observation.stderr,
            stdout_range: observation.stdout_range,
            stderr_range: observation.stderr_range,
            stdout_total_bytes: observation.stdout_total_bytes,
            stderr_total_bytes: observation.stderr_total_bytes,
            encoding: observation.encoding,
            exit_code: observation.exit_code,
            finished_at: observation.finished_at,
        },
    )
    .print();

    Ok(())
}
