//! Implementation of the `run` sub-command.
//!
//! Design decisions (from design.md):
//! - `run` spawns a short-lived front-end that forks a `_supervise` child.
//! - The supervisor continues logging stdout/stderr after `run` returns.
//! - If `--snapshot-after` elapses before `run` returns, a snapshot is
//!   included in the JSON response.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};
use ulid::Ulid;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{
    JobMeta, JobMetaJob, JobState, JobStateJob, JobStateResult, JobStatus, Response, RunData,
    Snapshot,
};

/// Options for the `run` sub-command.
#[derive(Debug)]
pub struct RunOpts<'a> {
    /// Command and arguments to execute.
    pub command: Vec<String>,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Milliseconds to wait before returning; 0 = return immediately.
    pub snapshot_after: u64,
    /// Number of tail lines to include in snapshot.
    pub tail_lines: u64,
    /// Max bytes for tail.
    pub max_bytes: u64,
    /// Timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the command.
    pub cwd: Option<&'a str>,
    /// Environment variables as KEY=VALUE strings.
    pub env_vars: Vec<String>,
    /// Paths to env files, applied in order.
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment (default: true).
    pub inherit_env: bool,
    /// Keys to mask in JSON output (values replaced with "***").
    pub mask: Vec<String>,
    /// Override full.log path; None = use job dir.
    pub log: Option<&'a str>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// If true, wait for the job to reach a terminal state before returning.
    /// The response will include exit_code, finished_at, and final_snapshot.
    pub wait: bool,
    /// Poll interval in milliseconds when `wait` is true.
    pub wait_poll_ms: u64,
}

impl<'a> Default for RunOpts<'a> {
    fn default() -> Self {
        RunOpts {
            command: vec![],
            root: None,
            snapshot_after: 10_000,
            tail_lines: 50,
            max_bytes: 65536,
            timeout_ms: 0,
            kill_after_ms: 0,
            cwd: None,
            env_vars: vec![],
            env_files: vec![],
            inherit_env: true,
            mask: vec![],
            log: None,
            progress_every_ms: 0,
            wait: false,
            wait_poll_ms: 200,
        }
    }
}

/// Maximum allowed value for `snapshot_after` in milliseconds (10 seconds).
const MAX_SNAPSHOT_AFTER_MS: u64 = 10_000;

/// Execute `run`: spawn job, possibly wait for snapshot, return JSON.
pub fn execute(opts: RunOpts) -> Result<()> {
    if opts.command.is_empty() {
        anyhow::bail!("no command specified for run");
    }

    let elapsed_start = std::time::Instant::now();

    let root = resolve_root(opts.root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create jobs root {}", root.display()))?;

    let job_id = Ulid::new().to_string();
    let created_at = now_rfc3339();

    // Extract only the key names from KEY=VALUE env var strings (values are not persisted).
    let env_keys: Vec<String> = opts
        .env_vars
        .iter()
        .map(|kv| kv.split('=').next().unwrap_or(kv.as_str()).to_string())
        .collect();

    // Apply masking: replace values of masked keys with "***" in env_vars for metadata.
    let masked_env_vars = mask_env_vars(&opts.env_vars, &opts.mask);

    let meta = JobMeta {
        job: JobMetaJob { id: job_id.clone() },
        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
        command: opts.command.clone(),
        created_at: created_at.clone(),
        root: root.display().to_string(),
        env_keys,
        env_vars: masked_env_vars.clone(),
        mask: opts.mask.clone(),
    };

    let job_dir = JobDir::create(&root, &job_id, &meta)?;
    info!(job_id = %job_id, "created job directory");

    // Determine the full.log path (may be overridden by --log).
    let full_log_path = if let Some(log) = opts.log {
        log.to_string()
    } else {
        job_dir.full_log_path().display().to_string()
    };

    // Pre-create empty log files so they exist before the supervisor starts.
    // This guarantees that `stdout.log`, `stderr.log`, and `full.log` are
    // present immediately after `run` returns, even if the supervisor has
    // not yet begun writing.
    for log_path in [
        job_dir.stdout_path(),
        job_dir.stderr_path(),
        job_dir.full_log_path(),
    ] {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("pre-create log file {}", log_path.display()))?;
    }

    // Spawn the supervisor (same binary, internal `_supervise` sub-command).
    let exe = std::env::current_exe().context("resolve current exe")?;
    let mut supervisor_cmd = Command::new(&exe);
    supervisor_cmd
        .arg("_supervise")
        .arg("--job-id")
        .arg(&job_id)
        .arg("--root")
        .arg(root.display().to_string())
        .arg("--full-log")
        .arg(&full_log_path);

    if opts.timeout_ms > 0 {
        supervisor_cmd
            .arg("--timeout")
            .arg(opts.timeout_ms.to_string());
    }
    if opts.kill_after_ms > 0 {
        supervisor_cmd
            .arg("--kill-after")
            .arg(opts.kill_after_ms.to_string());
    }
    if let Some(cwd) = opts.cwd {
        supervisor_cmd.arg("--cwd").arg(cwd);
    }
    for env_file in &opts.env_files {
        supervisor_cmd.arg("--env-file").arg(env_file);
    }
    for env_var in &opts.env_vars {
        supervisor_cmd.arg("--env").arg(env_var);
    }
    if !opts.inherit_env {
        supervisor_cmd.arg("--no-inherit-env");
    }
    // Note: masking is handled by `run` (meta.json + JSON response). The supervisor
    // receives the real env var values so the child process can use them as intended.
    if opts.progress_every_ms > 0 {
        supervisor_cmd
            .arg("--progress-every")
            .arg(opts.progress_every_ms.to_string());
    }

    supervisor_cmd
        .arg("--")
        .args(&opts.command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let supervisor = supervisor_cmd.spawn().context("spawn supervisor")?;

    let supervisor_pid = supervisor.id();
    debug!(supervisor_pid, "supervisor spawned");

    // Write initial state with supervisor PID so `status` can track it.
    // On Windows, this also pre-records the deterministic Job Object name
    // (AgentExec-{job_id}) so that callers can find it immediately after run returns.
    job_dir.init_state(supervisor_pid, &created_at)?;

    // On Windows, poll state.json until the supervisor confirms Job Object
    // assignment (state pid changes to child pid or state changes to "failed").
    // This handshake ensures `run` can detect Job Object assignment failures
    // before returning.  We wait up to 5 seconds (500 × 10ms intervals).
    #[cfg(windows)]
    {
        let handshake_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if let Ok(current_state) = job_dir.read_state() {
                // Supervisor has updated state once the pid changes from
                // supervisor_pid to the child pid (success), or state becomes
                // "failed" (Job Object assignment error).
                let supervisor_updated = current_state
                    .pid
                    .map(|p| p != supervisor_pid)
                    .unwrap_or(false)
                    || *current_state.status() == JobStatus::Failed;
                if supervisor_updated {
                    if *current_state.status() == JobStatus::Failed {
                        // Supervisor failed to assign the child to a Job Object
                        // and has already killed the child and updated state.json.
                        // Report failure to the caller.
                        anyhow::bail!(
                            "supervisor failed to assign child process to Job Object \
                             (Windows MUST requirement); see stderr for details"
                        );
                    }
                    debug!("supervisor confirmed Job Object assignment via state.json handshake");
                    break;
                }
            }
            if std::time::Instant::now() >= handshake_deadline {
                // Supervisor did not confirm within 5 seconds. Proceed with
                // the initial state (deterministic job name already written).
                debug!("supervisor handshake timed out; proceeding with initial state");
                break;
            }
        }
    }

    // Compute absolute paths for stdout.log and stderr.log.
    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    // Clamp snapshot_after to MAX_SNAPSHOT_AFTER_MS, but only when --wait is NOT set.
    // When --wait is set, we skip the snapshot_after phase entirely (the final_snapshot
    // from the --wait phase replaces it), so the clamp is irrelevant.
    let effective_snapshot_after = if opts.wait {
        // Skip the snapshot_after wait when --wait is active; the terminal-state
        // poll below will produce the definitive final_snapshot.
        0
    } else {
        opts.snapshot_after.min(MAX_SNAPSHOT_AFTER_MS)
    };

    // Start a single wait_start timer that spans both the snapshot_after phase
    // and the optional --wait phase so waited_ms reflects total wait time.
    let wait_start = std::time::Instant::now();

    // Optionally wait for snapshot and measure waited_ms.
    // Uses a polling loop so we can exit early when output is available
    // or the job has finished, rather than always sleeping the full duration.
    let snapshot = if effective_snapshot_after > 0 {
        debug!(ms = effective_snapshot_after, "polling for snapshot");
        let deadline = wait_start + std::time::Duration::from_millis(effective_snapshot_after);
        // Poll interval: 15ms gives good responsiveness without excessive CPU.
        let poll_interval = std::time::Duration::from_millis(15);
        loop {
            std::thread::sleep(poll_interval);
            // Early exit: job state changed from running (finished or failed).
            // Output availability alone does NOT cause early exit; we always
            // wait until the deadline when the job is still running.
            if let Ok(st) = job_dir.read_state()
                && *st.status() != JobStatus::Running
            {
                debug!("snapshot poll: job no longer running, exiting early");
                break;
            }
            // Exit when deadline is reached.
            if std::time::Instant::now() >= deadline {
                debug!("snapshot poll: deadline reached");
                break;
            }
        }
        let snap = build_snapshot(&job_dir, opts.tail_lines, opts.max_bytes);
        Some(snap)
    } else {
        None
    };

    // If --wait is set, wait for the job to reach a terminal state.
    // Unlike snapshot_after, there is no upper bound on wait time.
    // waited_ms will accumulate the full time spent (snapshot_after + wait phases).
    let (final_state, exit_code_opt, finished_at_opt, final_snapshot_opt) = if opts.wait {
        debug!("--wait: polling for terminal state");
        let poll = std::time::Duration::from_millis(opts.wait_poll_ms.max(1));
        loop {
            std::thread::sleep(poll);
            if let Ok(st) = job_dir.read_state()
                && *st.status() != JobStatus::Running
            {
                let snap = build_snapshot(&job_dir, opts.tail_lines, opts.max_bytes);
                let ec = st.exit_code();
                let fa = st.finished_at.clone();
                let state_str = st.status().as_str().to_string();
                break (state_str, ec, fa, Some(snap));
            }
        }
    } else {
        (JobStatus::Running.as_str().to_string(), None, None, None)
    };

    // waited_ms reflects the total time spent waiting (snapshot_after + --wait phases).
    let waited_ms = wait_start.elapsed().as_millis() as u64;

    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    let response = Response::new(
        "run",
        RunData {
            job_id,
            state: final_state,
            // Include masked env_vars in the JSON response so callers can inspect
            // which variables were set (with secret values replaced by "***").
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
    );
    response.print();
    Ok(())
}

fn build_snapshot(job_dir: &JobDir, tail_lines: u64, max_bytes: u64) -> Snapshot {
    let stdout = job_dir.read_tail_metrics("stdout.log", tail_lines, max_bytes);
    let stderr = job_dir.read_tail_metrics("stderr.log", tail_lines, max_bytes);
    Snapshot {
        truncated: stdout.truncated || stderr.truncated,
        encoding: "utf-8-lossy".to_string(),
        stdout_observed_bytes: stdout.observed_bytes,
        stderr_observed_bytes: stderr.observed_bytes,
        stdout_included_bytes: stdout.included_bytes,
        stderr_included_bytes: stderr.included_bytes,
        stdout_tail: stdout.tail,
        stderr_tail: stderr.tail,
    }
}

/// Options for the `_supervise` internal sub-command.
///
/// Masking is the responsibility of `run` (which writes masked values to meta.json
/// and includes them in the JSON response). The supervisor only needs the real
/// environment variable values to launch the child process correctly.
#[derive(Debug)]
pub struct SuperviseOpts<'a> {
    pub job_id: &'a str,
    pub root: &'a Path,
    pub command: &'a [String],
    /// Override full.log path; None = use job dir default.
    pub full_log: Option<&'a str>,
    /// Timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the child process.
    pub cwd: Option<&'a str>,
    /// Environment variables as KEY=VALUE strings (real values, not masked).
    pub env_vars: Vec<String>,
    /// Paths to env files, applied in order.
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment.
    pub inherit_env: bool,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
}

/// Mask the values of specified keys in a list of KEY=VALUE strings.
/// Keys listed in `mask_keys` will have their value replaced with "***".
fn mask_env_vars(env_vars: &[String], mask_keys: &[String]) -> Vec<String> {
    if mask_keys.is_empty() {
        return env_vars.to_vec();
    }
    env_vars
        .iter()
        .map(|s| {
            let (key, _val) = parse_env_var(s);
            if mask_keys.iter().any(|k| k == &key) {
                format!("{key}=***")
            } else {
                s.clone()
            }
        })
        .collect()
}

/// Parse a single KEY=VALUE or KEY= string into (key, value).
fn parse_env_var(s: &str) -> (String, String) {
    if let Some(pos) = s.find('=') {
        (s[..pos].to_string(), s[pos + 1..].to_string())
    } else {
        (s.to_string(), String::new())
    }
}

/// Load environment variables from a .env-style file.
/// Supports KEY=VALUE lines; lines starting with '#' and empty lines are ignored.
fn load_env_file(path: &str) -> Result<Vec<(String, String)>> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("read env-file {path}"))?;
    let mut vars = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        vars.push(parse_env_var(line));
    }
    Ok(vars)
}

/// Stream bytes from a child process output pipe to an individual log file and
/// to the shared `full.log`.
///
/// Reads byte chunks (not lines) so that output without a trailing newline is
/// still captured in the individual log immediately.  The `full.log` format
/// `"<RFC3339> [LABEL] <line>"` is maintained via a line-accumulation buffer:
/// bytes are appended to the buffer until a newline is found, at which point a
/// formatted line is written to `full.log`.  Any remaining bytes at EOF are
/// flushed as a final line.
///
/// This helper is used by both the stdout and stderr monitoring threads inside
/// [`supervise`], replacing the previously duplicated per-stream implementations.
/// Buffer size (8192 bytes) and newline-split logic are preserved unchanged.
fn stream_to_logs<R>(
    stream: R,
    log_path: &std::path::Path,
    full_log: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
    label: &str,
) where
    R: std::io::Read,
{
    use std::io::Write;
    let mut log_file = std::fs::File::create(log_path).expect("create stream log file in thread");
    let mut stream = stream;
    let mut buf = [0u8; 8192];
    // Incomplete-line buffer for full.log formatting.
    let mut line_buf: Vec<u8> = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let chunk = &buf[..n];
                // Write raw bytes to the individual log (captures partial lines too).
                let _ = log_file.write_all(chunk);
                // Accumulate bytes for full.log line formatting.
                for &b in chunk {
                    if b == b'\n' {
                        let line = String::from_utf8_lossy(&line_buf);
                        if let Ok(mut fl) = full_log.lock() {
                            let ts = now_rfc3339();
                            let _ = writeln!(fl, "{ts} [{label}] {line}");
                        }
                        line_buf.clear();
                    } else {
                        line_buf.push(b);
                    }
                }
            }
            Err(_) => break,
        }
    }
    // Flush any remaining incomplete line to full.log.
    if !line_buf.is_empty() {
        let line = String::from_utf8_lossy(&line_buf);
        if let Ok(mut fl) = full_log.lock() {
            let ts = now_rfc3339();
            let _ = writeln!(fl, "{ts} [{label}] {line}");
        }
    }
}

/// Internal supervisor sub-command.
///
/// Runs the target command, streams stdout/stderr to individual log files
/// (`stdout.log`, `stderr.log`) **and** to the combined `full.log`, then
/// updates `state.json` when the process finishes.
///
/// On Windows, the child process is assigned to a named Job Object so that
/// the entire process tree can be terminated with a single `kill` call.
/// The Job Object name is recorded in `state.json` as `windows_job_name`.
pub fn supervise(opts: SuperviseOpts) -> Result<()> {
    use std::sync::{Arc, Mutex};

    let job_id = opts.job_id;
    let root = opts.root;
    let command = opts.command;

    if command.is_empty() {
        anyhow::bail!("supervisor: no command");
    }

    let job_dir = JobDir::open(root, job_id)?;

    // Read meta.json to get the started_at timestamp.
    let meta = job_dir.read_meta()?;
    let started_at = meta.created_at.clone();

    // Determine full.log path.
    let full_log_path = if let Some(p) = opts.full_log {
        std::path::PathBuf::from(p)
    } else {
        job_dir.full_log_path()
    };

    // Create the full.log file (shared between stdout/stderr threads).
    // Ensure parent directories exist for custom paths.
    if let Some(parent) = full_log_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir for full.log: {}", parent.display()))?;
    }
    let full_log_file = std::fs::File::create(&full_log_path).context("create full.log")?;
    let full_log = Arc::new(Mutex::new(full_log_file));

    // Build the child environment.
    let mut child_cmd = Command::new(&command[0]);
    child_cmd.args(&command[1..]);

    if opts.inherit_env {
        // Start with the current environment (default).
    } else {
        child_cmd.env_clear();
    }

    // Apply env files in order.
    for env_file in &opts.env_files {
        let vars = load_env_file(env_file)?;
        for (k, v) in vars {
            child_cmd.env(&k, &v);
        }
    }

    // Apply --env KEY=VALUE overrides (applied after env-files).
    for env_var in &opts.env_vars {
        let (k, v) = parse_env_var(env_var);
        child_cmd.env(&k, &v);
    }

    // Set working directory if specified.
    if let Some(cwd) = opts.cwd {
        child_cmd.current_dir(cwd);
    }

    // Spawn the child with piped stdout/stderr so we can tee to logs.
    let mut child = child_cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("supervisor: spawn child")?;

    let pid = child.id();
    info!(job_id, pid, "child process started");

    // On Windows, assign child to a named Job Object for process-tree management.
    // The job name is derived from the job_id so that `kill` can look it up.
    // Assignment is a MUST requirement on Windows: if it fails, the supervisor
    // kills the child process and updates state.json to "failed" before returning
    // an error, so that the run front-end (which may have already returned) can
    // detect the failure via state.json on next poll.
    #[cfg(windows)]
    let windows_job_name = {
        match assign_to_job_object(job_id, pid) {
            Ok(name) => Some(name),
            Err(e) => {
                // Job Object assignment failed. Per design.md this is a MUST
                // requirement on Windows. Kill the child process and update
                // state.json to "failed" so the run front-end can detect it.
                let kill_err = child.kill();
                let _ = child.wait(); // reap to avoid zombies

                let failed_state = JobState {
                    job: JobStateJob {
                        id: job_id.to_string(),
                        status: JobStatus::Failed,
                        started_at: started_at.clone(),
                    },
                    result: JobStateResult {
                        exit_code: None,
                        signal: None,
                        duration_ms: None,
                    },
                    pid: Some(pid),
                    finished_at: Some(now_rfc3339()),
                    updated_at: now_rfc3339(),
                    windows_job_name: None,
                };
                // Best-effort: if writing state fails, we still propagate the
                // original assignment error.
                let _ = job_dir.write_state(&failed_state);

                if let Err(ke) = kill_err {
                    return Err(anyhow::anyhow!(
                        "supervisor: failed to assign pid {pid} to Job Object \
                         (Windows MUST requirement): {e}; also failed to kill child: {ke}"
                    ));
                }
                return Err(anyhow::anyhow!(
                    "supervisor: failed to assign pid {pid} to Job Object \
                     (Windows MUST requirement); child process was killed; \
                     consider running outside a nested Job Object environment: {e}"
                ));
            }
        }
    };
    #[cfg(not(windows))]
    let windows_job_name: Option<String> = None;

    // Update state.json with real child PID and Windows Job Object name.
    // On Windows, windows_job_name is always Some at this point (guaranteed
    // by the MUST requirement above), so state.json will always contain the
    // Job Object identifier while the job is running.
    let state = JobState {
        job: JobStateJob {
            id: job_id.to_string(),
            status: JobStatus::Running,
            started_at: started_at.clone(),
        },
        result: JobStateResult {
            exit_code: None,
            signal: None,
            duration_ms: None,
        },
        pid: Some(pid),
        finished_at: None,
        updated_at: now_rfc3339(),
        windows_job_name,
    };
    job_dir.write_state(&state)?;

    let child_start_time = std::time::Instant::now();

    // Take stdout/stderr handles before moving child.
    let child_stdout = child.stdout.take().expect("child stdout piped");
    let child_stderr = child.stderr.take().expect("child stderr piped");

    // Thread: read stdout, write to stdout.log and full.log.
    let stdout_log_path = job_dir.stdout_path();
    let full_log_stdout = Arc::clone(&full_log);
    let t_stdout = std::thread::spawn(move || {
        stream_to_logs(child_stdout, &stdout_log_path, full_log_stdout, "STDOUT");
    });

    // Thread: read stderr, write to stderr.log and full.log.
    let stderr_log_path = job_dir.stderr_path();
    let full_log_stderr = Arc::clone(&full_log);
    let t_stderr = std::thread::spawn(move || {
        stream_to_logs(child_stderr, &stderr_log_path, full_log_stderr, "STDERR");
    });

    // Timeout / kill-after / progress-every handling.
    // We spawn a watcher thread to handle timeout and periodic state.json updates.
    let timeout_ms = opts.timeout_ms;
    let kill_after_ms = opts.kill_after_ms;
    let progress_every_ms = opts.progress_every_ms;
    let state_path = job_dir.state_path();
    let job_id_str = job_id.to_string();

    // Use an atomic flag to signal the watcher thread when the child has exited.
    use std::sync::atomic::{AtomicBool, Ordering};
    let child_done = Arc::new(AtomicBool::new(false));

    let watcher = if timeout_ms > 0 || progress_every_ms > 0 {
        let state_path_clone = state_path.clone();
        let child_done_clone = Arc::clone(&child_done);
        Some(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let timeout_dur = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };
            let progress_dur = if progress_every_ms > 0 {
                Some(std::time::Duration::from_millis(progress_every_ms))
            } else {
                None
            };

            let poll_interval = std::time::Duration::from_millis(100);

            loop {
                std::thread::sleep(poll_interval);

                // Exit the watcher loop if the child process has finished.
                if child_done_clone.load(Ordering::Relaxed) {
                    break;
                }

                let elapsed = start.elapsed();

                // Check for timeout.
                if let Some(td) = timeout_dur
                    && elapsed >= td
                {
                    info!(job_id = %job_id_str, "timeout reached, sending SIGTERM");
                    // Send SIGTERM.
                    #[cfg(unix)]
                    {
                        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
                    }
                    // If kill_after > 0, wait kill_after ms then SIGKILL.
                    if kill_after_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(kill_after_ms));
                        info!(job_id = %job_id_str, "kill-after elapsed, sending SIGKILL");
                        #[cfg(unix)]
                        {
                            unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
                        }
                    } else {
                        // Immediate SIGKILL.
                        #[cfg(unix)]
                        {
                            unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
                        }
                    }
                    break;
                }

                // Progress-every: update updated_at periodically.
                if let Some(pd) = progress_dur {
                    let elapsed_ms = elapsed.as_millis() as u64;
                    let pd_ms = pd.as_millis() as u64;
                    let poll_ms = poll_interval.as_millis() as u64;
                    if elapsed_ms % pd_ms < poll_ms {
                        // Read, update updated_at, write back.
                        if let Ok(raw) = std::fs::read(&state_path_clone)
                            && let Ok(mut st) =
                                serde_json::from_slice::<crate::schema::JobState>(&raw)
                        {
                            st.updated_at = now_rfc3339();
                            if let Ok(s) = serde_json::to_string_pretty(&st) {
                                let _ = std::fs::write(&state_path_clone, s);
                            }
                        }
                    }
                }
            }
        }))
    } else {
        None
    };

    // Wait for child to finish.
    let exit_status = child.wait().context("wait for child")?;

    // Signal the watcher that the child has finished so it can exit its loop.
    child_done.store(true, Ordering::Relaxed);

    // Join logging threads.
    let _ = t_stdout.join();
    let _ = t_stderr.join();

    // Join watcher if present.
    if let Some(w) = watcher {
        let _ = w.join();
    }

    let duration_ms = child_start_time.elapsed().as_millis() as u64;
    let exit_code = exit_status.code();
    let finished_at = now_rfc3339();

    let state = JobState {
        job: JobStateJob {
            id: job_id.to_string(),
            status: JobStatus::Exited, // non-zero exit still "exited"
            started_at,
        },
        result: JobStateResult {
            exit_code,
            signal: None,
            duration_ms: Some(duration_ms),
        },
        pid: Some(pid),
        finished_at: Some(finished_at),
        updated_at: now_rfc3339(),
        windows_job_name: None, // not needed after process exits
    };
    job_dir.write_state(&state)?;
    info!(job_id, ?exit_code, "child process finished");
    Ok(())
}

/// Public alias so other modules can call the timestamp helper.
pub fn now_rfc3339_pub() -> String {
    now_rfc3339()
}

fn now_rfc3339() -> String {
    // Use a simple approach that works without chrono.
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_rfc3339(d.as_secs())
}

fn format_rfc3339(secs: u64) -> String {
    // Manual conversion of Unix timestamp to UTC date-time string.
    let mut s = secs;
    let seconds = s % 60;
    s /= 60;
    let minutes = s % 60;
    s /= 60;
    let hours = s % 24;
    s /= 24;

    // Days since 1970-01-01
    let mut days = s;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            month = i;
            break;
        }
        days -= d;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Windows-only: create a named Job Object and assign the given child process
/// to it so that the entire process tree can be terminated via `kill`.
///
/// The Job Object is named `"AgentExec-{job_id}"`. This name is stored in
/// `state.json` so that future `kill` invocations can open the same Job Object
/// by name and call `TerminateJobObject` to stop the whole tree.
///
/// Returns `Ok(name)` on success.  Returns `Err` on failure — the caller
/// (`supervise`) treats failure as a fatal error because reliable process-tree
/// management is a Windows MUST requirement (design.md).
#[cfg(windows)]
fn assign_to_job_object(job_id: &str, pid: u32) -> Result<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};
    use windows::core::HSTRING;

    let job_name = format!("AgentExec-{job_id}");
    let hname = HSTRING::from(job_name.as_str());

    unsafe {
        // Open the child process handle (needed for AssignProcessToJobObject).
        let proc_handle =
            OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, false, pid).map_err(|e| {
                anyhow::anyhow!(
                    "supervisor: OpenProcess(pid={pid}) failed — cannot assign to Job Object: {e}"
                )
            })?;

        // Create a named Job Object.
        let job = match CreateJobObjectW(None, &hname) {
            Ok(h) => h,
            Err(e) => {
                let _ = CloseHandle(proc_handle);
                return Err(anyhow::anyhow!(
                    "supervisor: CreateJobObjectW({job_name}) failed: {e}"
                ));
            }
        };

        // Assign the child process to the Job Object.
        // This can fail if the process is already in another job (e.g. CI/nested).
        // Per design.md, assignment is a MUST on Windows — failure is a fatal error.
        if let Err(e) = AssignProcessToJobObject(job, proc_handle) {
            let _ = CloseHandle(job);
            let _ = CloseHandle(proc_handle);
            return Err(anyhow::anyhow!(
                "supervisor: AssignProcessToJobObject(pid={pid}) failed \
                 (process may already belong to another Job Object, e.g. in a CI environment): {e}"
            ));
        }

        // Keep job handle open for the lifetime of the supervisor so the Job
        // Object remains valid. We intentionally do NOT close it here.
        // The OS will close it automatically when the supervisor exits.
        // (We close proc_handle since we only needed it for assignment.)
        let _ = CloseHandle(proc_handle);
        // Note: job handle is intentionally leaked here to keep the Job Object alive.
        // The handle will be closed when the supervisor process exits.
        std::mem::forget(job);
    }

    info!(job_id, name = %job_name, "supervisor: child assigned to Job Object");
    Ok(job_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_epoch() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_known_date() {
        // 2024-01-01T00:00:00Z = 1704067200
        assert_eq!(format_rfc3339(1704067200), "2024-01-01T00:00:00Z");
    }
}
