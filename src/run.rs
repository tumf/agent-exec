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
}

impl<'a> Default for RunOpts<'a> {
    fn default() -> Self {
        RunOpts {
            command: vec![],
            root: None,
            snapshot_after: 0,
            tail_lines: 50,
            max_bytes: 65536,
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

    // Spawn the supervisor (same binary, internal `_supervise` sub-command).
    let exe = std::env::current_exe().context("resolve current exe")?;
    let supervisor = Command::new(&exe)
        .arg("_supervise")
        .arg("--job-id")
        .arg(&job_id)
        .arg("--root")
        .arg(root.display().to_string())
        .arg("--")
        .args(&opts.command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawn supervisor")?;

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
    Snapshot {
        stdout_tail: job_dir.tail_log("stdout.log", tail_lines, max_bytes),
        stderr_tail: job_dir.tail_log("stderr.log", tail_lines, max_bytes),
        encoding: "utf-8-lossy".to_string(),
    }
}

/// Internal supervisor sub-command.
///
/// Runs the target command, streams stdout/stderr to individual log files
/// (`stdout.log`, `stderr.log`) **and** to the combined `full.log`, then
/// updates `state.json` when the process finishes.
pub fn supervise(job_id: &str, root: &Path, command: &[String]) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex};

    if command.is_empty() {
        anyhow::bail!("supervisor: no command");
    }

    let job_dir = JobDir::open(root, job_id)?;

    // Create the full.log file (shared between stdout/stderr threads).
    let full_log_file =
        std::fs::File::create(job_dir.full_log_path()).context("create full.log")?;
    let full_log = Arc::new(Mutex::new(full_log_file));

    // Spawn the child with piped stdout/stderr so we can tee to logs.
    let mut child = Command::new(&command[0])
        .args(&command[1..])
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
                let _ = writeln!(fl, "{line}");
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
                let _ = writeln!(fl, "{line}");
            }
        }
    });

    // Wait for child to finish.
    let exit_status = child.wait().context("wait for child")?;

    // Join logging threads.
    let _ = t_stdout.join();
    let _ = t_stderr.join();

    let exit_code = exit_status.code();
    let finished_at = now_rfc3339();

    let state = JobState {
        state: JobStatus::Exited, // non-zero exit still "exited"
        pid: Some(pid),
        exit_code,
        finished_at: Some(finished_at),
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
