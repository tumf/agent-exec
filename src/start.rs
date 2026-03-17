//! Implementation of the `start` sub-command.
//!
//! `start` launches a previously `create`d job.  It reads the persisted
//! execution definition from `meta.json`, validates that the job is in
//! `created` state, and then spawns the supervisor process.

use anyhow::Result;
use tracing::info;

use crate::jobstore::{InvalidJobState, JobDir, resolve_root};
use crate::run::{
    SnapshotWaitOpts, SpawnSupervisorParams, mask_env_vars, run_snapshot_wait,
    spawn_supervisor_process,
};
use crate::schema::{JobStatus, Response, RunData};

/// Options for the `start` sub-command.
#[derive(Debug)]
pub struct StartOpts<'a> {
    /// Job ID of a previously created job.
    pub job_id: &'a str,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Milliseconds to wait before returning; 0 = return immediately.
    pub snapshot_after: u64,
    /// Number of tail lines to include in snapshot.
    pub tail_lines: u64,
    /// Max bytes for tail.
    pub max_bytes: u64,
    /// If true, wait for the job to reach a terminal state before returning.
    pub wait: bool,
    /// Poll interval in milliseconds when `wait` is true.
    pub wait_poll_ms: u64,
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

    // The real (unmasked) env vars are NOT stored in meta.json for security.
    // Only masked display values are persisted. For the supervisor we need to
    // reconstruct the env from env_files (re-read at start time) only.
    // Direct KEY=VALUE pairs from --env were stored as masked; they will be
    // applied via env_files if they came from files, or not re-applied if they
    // were direct key=value args (per design: --env is durable non-secret config,
    // but values are intentionally not persisted in plain text).
    //
    // Practical note: we pass the masked env_vars to the supervisor; the
    // supervisor will apply them as-is (masked values like "***" will be set
    // in the child environment). Users should use env_files for actual secrets.
    // The contract for `--env` in the create/start lifecycle treats these as
    // durable, non-secret configuration.

    let (supervisor_pid, started_at) = spawn_supervisor_process(
        &job_dir,
        SpawnSupervisorParams {
            job_id: opts.job_id.to_string(),
            root: root.clone(),
            full_log_path: full_log_path.clone(),
            timeout_ms: meta.timeout_ms,
            kill_after_ms: meta.kill_after_ms,
            cwd: meta.cwd.clone(),
            env_vars: meta.env_vars.clone(), // masked values — see note above
            env_files: meta.env_files.clone(),
            inherit_env: meta.inherit_env,
            progress_every_ms: meta.progress_every_ms,
            notify_command: meta.notification.as_ref().and_then(|n| n.notify_command.clone()),
            notify_file: meta.notification.as_ref().and_then(|n| n.notify_file.clone()),
            shell_wrapper,
            command: meta.command.clone(),
        },
    )?;

    info!(job_id = %opts.job_id, supervisor_pid, started_at = %started_at, "job started");

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    let elapsed_start = std::time::Instant::now();

    let (final_state, exit_code_opt, finished_at_opt, snapshot, final_snapshot_opt, waited_ms) =
        run_snapshot_wait(
            &job_dir,
            &SnapshotWaitOpts {
                snapshot_after: opts.snapshot_after,
                tail_lines: opts.tail_lines,
                max_bytes: opts.max_bytes,
                wait: opts.wait,
                wait_poll_ms: opts.wait_poll_ms,
            },
        );

    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    // The masked env_vars included in the response match what was persisted in meta.
    let masked_env_vars = mask_env_vars(&meta.env_vars, &meta.mask);

    Response::new(
        "start",
        RunData {
            job_id: opts.job_id.to_string(),
            state: final_state,
            env_vars: masked_env_vars,
            snapshot,
            stdout_log_path,
            stderr_log_path,
            waited_ms,
            elapsed_ms,
            exit_code: exit_code_opt,
            finished_at: finished_at_opt,
            final_snapshot: final_snapshot_opt,
        },
    )
    .print();

    Ok(())
}
