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
    // On Windows, this also pre-records the deterministic Job Object name
    // (AgentExec-{job_id}) so that callers can find it immediately after run returns.
    job_dir.init_state(supervisor_pid)?;

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
                    || current_state.state == JobStatus::Failed;
                if supervisor_updated {
                    if current_state.state == JobStatus::Failed {
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
///
/// On Windows, the child process is assigned to a named Job Object so that
/// the entire process tree can be terminated with a single `kill` call.
/// The Job Object name is recorded in `state.json` as `windows_job_name`.
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
                    state: JobStatus::Failed,
                    pid: Some(pid),
                    exit_code: None,
                    finished_at: Some(now_rfc3339()),
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
        state: JobStatus::Running,
        pid: Some(pid),
        exit_code: None,
        finished_at: None,
        windows_job_name,
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
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
    use windows::core::HSTRING;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

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
