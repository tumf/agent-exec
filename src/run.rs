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

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{JobMeta, JobState, JobStatus, Response, RunData, Snapshot};

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
}

impl<'a> Default for RunOpts<'a> {
    fn default() -> Self {
        RunOpts {
            command: vec![],
            root: None,
            snapshot_after: 0,
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
        }
    }
}

/// Execute `run`: spawn job, possibly wait for snapshot, return JSON.
pub fn execute(opts: RunOpts) -> Result<()> {
    if opts.command.is_empty() {
        anyhow::bail!("no command specified for run");
    }

    let root = resolve_root(opts.root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create jobs root {}", root.display()))?;

    let job_id = Ulid::new().to_string();
    let started_at = now_rfc3339();

    let meta = JobMeta {
        job_id: job_id.clone(),
        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
        command: opts.command.clone(),
        started_at: started_at.clone(),
        root: root.display().to_string(),
    };

    let job_dir = JobDir::create(&root, &job_id, &meta)?;
    info!(job_id = %job_id, "created job directory");

    // Determine the full.log path (may be overridden by --log).
    let full_log_path = if let Some(log) = opts.log {
        log.to_string()
    } else {
        job_dir.full_log_path().display().to_string()
    };

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
    for key in &opts.mask {
        supervisor_cmd.arg("--mask").arg(key);
    }
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
    job_dir.init_state(supervisor_pid)?;

    // Optionally wait for snapshot.
    let snapshot = if opts.snapshot_after > 0 {
        debug!(ms = opts.snapshot_after, "waiting for snapshot");
        std::thread::sleep(std::time::Duration::from_millis(opts.snapshot_after));
        Some(build_snapshot(&job_dir, opts.tail_lines, opts.max_bytes))
    } else {
        None
    };

    let response = Response::new(
        "run",
        RunData {
            job_id,
            state: JobStatus::Running.as_str().to_string(),
            snapshot,
        },
    );
    response.print();
    Ok(())
}

fn build_snapshot(job_dir: &JobDir, tail_lines: u64, max_bytes: u64) -> Snapshot {
    let (stdout_tail, stdout_truncated) =
        job_dir.tail_log_with_truncated("stdout.log", tail_lines, max_bytes);
    let (stderr_tail, stderr_truncated) =
        job_dir.tail_log_with_truncated("stderr.log", tail_lines, max_bytes);
    Snapshot {
        stdout_tail,
        stderr_tail,
        truncated: stdout_truncated || stderr_truncated,
        encoding: "utf-8-lossy".to_string(),
    }
}

/// Options for the `_supervise` internal sub-command.
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
    /// Environment variables as KEY=VALUE strings.
    pub env_vars: Vec<String>,
    /// Paths to env files, applied in order.
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment.
    pub inherit_env: bool,
    /// Keys to mask in output.
    pub mask: Vec<String>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
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

/// Internal supervisor sub-command.
///
/// Runs the target command, streams stdout/stderr to individual log files
/// (`stdout.log`, `stderr.log`) **and** to the combined `full.log`, then
/// updates `state.json` when the process finishes.
pub fn supervise(opts: SuperviseOpts) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex};

    let job_id = opts.job_id;
    let root = opts.root;
    let command = opts.command;

    if command.is_empty() {
        anyhow::bail!("supervisor: no command");
    }

    let job_dir = JobDir::open(root, job_id)?;

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

    // Update state.json with real child PID.
    let state = JobState {
        state: JobStatus::Running,
        pid: Some(pid),
        exit_code: None,
        finished_at: None,
        updated_at: Some(now_rfc3339()),
    };
    job_dir.write_state(&state)?;

    // Take stdout/stderr handles before moving child.
    let child_stdout = child.stdout.take().expect("child stdout piped");
    let child_stderr = child.stderr.take().expect("child stderr piped");

    // Thread: read stdout, write to stdout.log and full.log.
    let stdout_log_path = job_dir.stdout_path();
    let full_log_stdout = Arc::clone(&full_log);
    let t_stdout = std::thread::spawn(move || {
        let mut stdout_file =
            std::fs::File::create(&stdout_log_path).expect("create stdout.log in thread");
        let reader = BufReader::new(child_stdout);
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            let _ = writeln!(stdout_file, "{line}");
            if let Ok(mut fl) = full_log_stdout.lock() {
                // full.log format: "<RFC3339> [STDOUT] <line>"
                let ts = now_rfc3339();
                let _ = writeln!(fl, "{ts} [STDOUT] {line}");
            }
        }
    });

    // Thread: read stderr, write to stderr.log and full.log.
    let stderr_log_path = job_dir.stderr_path();
    let full_log_stderr = Arc::clone(&full_log);
    let t_stderr = std::thread::spawn(move || {
        let mut stderr_file =
            std::fs::File::create(&stderr_log_path).expect("create stderr.log in thread");
        let reader = BufReader::new(child_stderr);
        for line in reader.lines() {
            let line = line.unwrap_or_default();
            let _ = writeln!(stderr_file, "{line}");
            if let Ok(mut fl) = full_log_stderr.lock() {
                // full.log format: "<RFC3339> [STDERR] <line>"
                let ts = now_rfc3339();
                let _ = writeln!(fl, "{ts} [STDERR] {line}");
            }
        }
    });

    // Timeout / kill-after / progress-every handling.
    // We spawn a watcher thread to handle timeout and periodic state.json updates.
    let timeout_ms = opts.timeout_ms;
    let kill_after_ms = opts.kill_after_ms;
    let progress_every_ms = opts.progress_every_ms;
    let state_path = job_dir.state_path();
    let job_id_str = job_id.to_string();

    let watcher = if timeout_ms > 0 || progress_every_ms > 0 {
        let state_path_clone = state_path.clone();
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
                let elapsed = start.elapsed();

                // Check for timeout.
                if let Some(td) = timeout_dur {
                    if elapsed >= td {
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
                }

                // Progress-every: update updated_at periodically.
                if let Some(pd) = progress_dur {
                    let elapsed_ms = elapsed.as_millis() as u64;
                    let pd_ms = pd.as_millis() as u64;
                    let poll_ms = poll_interval.as_millis() as u64;
                    if elapsed_ms % pd_ms < poll_ms {
                        // Read, update updated_at, write back.
                        if let Ok(raw) = std::fs::read(&state_path_clone) {
                            if let Ok(mut st) =
                                serde_json::from_slice::<crate::schema::JobState>(&raw)
                            {
                                st.updated_at = Some(now_rfc3339());
                                if let Ok(s) = serde_json::to_string_pretty(&st) {
                                    let _ = std::fs::write(&state_path_clone, s);
                                }
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

    // Join logging threads.
    let _ = t_stdout.join();
    let _ = t_stderr.join();

    // Join watcher if present.
    if let Some(w) = watcher {
        let _ = w.join();
    }

    let exit_code = exit_status.code();
    let finished_at = now_rfc3339();

    let state = JobState {
        state: JobStatus::Exited, // non-zero exit still "exited"
        pid: Some(pid),
        exit_code,
        finished_at: Some(finished_at.clone()),
        updated_at: Some(finished_at),
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
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
