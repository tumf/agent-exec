//! Implementation of the `restart` sub-command.
//!
//! `restart` reuses an existing job directory and persisted `meta.json` definition
//! while replacing the current process, if any, with a fresh supervisor launch.

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::jobstore::{InvalidJobState, JobDir, resolve_root};
use crate::run::{
    SpawnSupervisorParams, mask_env_vars, observe_inline_output, spawn_supervisor_process,
};
use crate::schema::{JobStatus, Response, RunData};

const TERMINATION_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);
const TERMINATION_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Options for the `restart` sub-command.
#[derive(Debug)]
pub struct RestartOpts<'a> {
    /// Job ID of an existing job.
    pub job_id: &'a str,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Signal used to terminate a currently running process tree.
    pub signal: &'a str,
    /// Disable best-effort auto-GC for this invocation.
    pub no_auto_gc: bool,
    /// Optional auto-GC retention override.
    pub auto_gc_older_than: Option<String>,
    /// Optional auto-GC max-jobs override.
    pub auto_gc_max_jobs: Option<u64>,
    /// Optional auto-GC max-bytes override.
    pub auto_gc_max_bytes: Option<u64>,
    /// Base auto-GC settings resolved from config/defaults.
    pub auto_gc_config: crate::gc::AutoGcConfig,
    /// Wait for inline output observation before returning.
    pub wait: bool,
    /// Maximum wait duration in seconds for inline observation.
    pub until_seconds: u64,
    /// Wait indefinitely for terminal state / observation budget.
    pub forever: bool,
    /// Maximum bytes to include from the head of each stream.
    pub max_bytes: u64,
}

/// Execute `restart`: replace an existing job's current run and return JSON.
pub fn execute(opts: RestartOpts) -> Result<()> {
    let elapsed_start = std::time::Instant::now();
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let meta = job_dir.read_meta()?;
    if meta.job_id() != job_dir.job_id {
        return Err(anyhow::Error::new(InvalidJobState(format!(
            "job {} metadata identity mismatch: meta.json has {}",
            job_dir.job_id,
            meta.job_id()
        ))));
    }

    let state = job_dir.read_state()?;
    info!(
        job_id = %job_dir.job_id,
        state = %state.status().as_str(),
        "restarting job"
    );

    if *state.status() == JobStatus::Running {
        terminate_running_job(&job_dir, opts.signal)?;
    }

    reset_per_run_artifacts(&job_dir)?;

    let full_log_path = job_dir.full_log_path().display().to_string();
    let shell_wrapper = meta
        .shell_wrapper
        .clone()
        .unwrap_or_else(crate::config::default_shell_wrapper);

    let (supervisor_pid, started_at) = spawn_supervisor_process(
        &job_dir,
        SpawnSupervisorParams {
            job_id: job_dir.job_id.clone(),
            root: root.clone(),
            full_log_path,
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

    info!(
        job_id = %job_dir.job_id,
        supervisor_pid,
        started_at = %started_at,
        "job restarted"
    );

    if !opts.no_auto_gc {
        let mut auto_cfg = opts.auto_gc_config.clone();
        if let Some(v) = opts.auto_gc_older_than {
            auto_cfg.older_than = v;
        }
        if let Some(v) = opts.auto_gc_max_jobs {
            auto_cfg.max_jobs = usize::try_from(v).ok();
        }
        if let Some(v) = opts.auto_gc_max_bytes {
            auto_cfg.max_bytes = Some(v);
        }
        crate::gc::maybe_run_auto_gc(&root, &auto_cfg);
    }

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();
    let masked_env_vars = mask_env_vars(&meta.env_vars_runtime, &meta.mask);
    let observation = observe_inline_output(
        &job_dir,
        opts.wait,
        opts.until_seconds,
        opts.forever,
        opts.max_bytes,
    )?;
    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    Response::new(
        "restart",
        RunData {
            job_id: job_dir.job_id.clone(),
            state: observation.state,
            tags: meta.tags.clone(),
            env_vars: masked_env_vars,
            stdout_log_path,
            stderr_log_path,
            elapsed_ms,
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
            signal: observation.signal,
            duration_ms: observation.duration_ms,
        },
    )
    .print();

    Ok(())
}

fn terminate_running_job(job_dir: &JobDir, signal: &str) -> Result<()> {
    info!(job_id = %job_dir.job_id, signal, "terminating running job before restart");

    let original_pid = job_dir.read_state()?.pid;
    let signal_result = crate::kill::execute_inner(crate::kill::KillOpts {
        job_id: &job_dir.job_id,
        root: job_dir.path.parent().and_then(|p| p.to_str()),
        signal,
        no_wait: false,
    })?;

    if matches!(signal_result.state.as_deref(), Some("running")) {
        warn!(
            job_id = %job_dir.job_id,
            signal,
            "restart termination observation still reported running; escalating to KILL"
        );
        crate::kill::execute_inner(crate::kill::KillOpts {
            job_id: &job_dir.job_id,
            root: job_dir.path.parent().and_then(|p| p.to_str()),
            signal: "KILL",
            no_wait: false,
        })?;
    }

    let deadline = std::time::Instant::now() + TERMINATION_BUDGET;
    loop {
        let current = job_dir.read_state()?;
        let state_is_terminal = !current.status().is_non_terminal();
        let original_process_gone = original_pid.map(process_is_gone).unwrap_or(true);
        if state_is_terminal && original_process_gone {
            info!(
                job_id = %job_dir.job_id,
                state = %current.status().as_str(),
                original_pid = ?original_pid,
                "old job run reached terminal state before restart relaunch"
            );
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(anyhow::Error::new(InvalidJobState(format!(
                "job {} did not terminate within restart budget (state_terminal={}, original_pid_gone={})",
                job_dir.job_id, state_is_terminal, original_process_gone
            ))));
        }
        std::thread::sleep(TERMINATION_POLL_INTERVAL);
    }
}

fn process_is_gone(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: kill(pid, 0) does not send a signal; it only probes process existence.
        let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if ret == 0 {
            return false;
        }
        std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    }
    #[cfg(not(unix))]
    {
        // On non-Unix platforms the shared kill path owns process-tree handling;
        // state observation is the portable confirmation available here.
        let _ = pid;
        true
    }
}

fn reset_per_run_artifacts(job_dir: &JobDir) -> Result<()> {
    for path in [
        job_dir.stdout_path(),
        job_dir.stderr_path(),
        job_dir.full_log_path(),
    ] {
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format!("truncate per-run artifact {}", path.display()))?;
    }

    let completion_event_path = job_dir.completion_event_path();
    match std::fs::remove_file(&completion_event_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e)
                .with_context(|| format!("remove stale {}", completion_event_path.display()));
        }
    }

    Ok(())
}
