//! Implementation of the `kill` sub-command.
//!
//! Signals supported: TERM, INT, KILL (case-insensitive).
//! On Windows, all signals map to TerminateProcess (Job Object termination).

use anyhow::Result;
use tracing::info;

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{JobState, JobStateJob, JobStateResult, JobStatus, KillData, Response};

/// Options for the `kill` sub-command.
#[derive(Debug)]
pub struct KillOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// Signal name: TERM | INT | KILL (default: TERM).
    pub signal: &'a str,
}

impl<'a> Default for KillOpts<'a> {
    fn default() -> Self {
        KillOpts {
            job_id: "",
            root: None,
            signal: "TERM",
        }
    }
}

/// Execute `kill`: send signal and emit JSON.
pub fn execute(opts: KillOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let state = job_dir.read_state()?;
    let signal_upper = opts.signal.to_uppercase();

    if *state.status() != JobStatus::Running {
        // Already stopped — no-op but still emit JSON.
        let response = Response::new(
            "kill",
            KillData {
                job_id: opts.job_id.to_string(),
                signal: signal_upper,
            },
        );
        response.print();
        return Ok(());
    }

    if let Some(pid) = state.pid {
        send_signal(pid, &signal_upper)?;
        info!(job_id = %opts.job_id, pid, signal = %signal_upper, "signal sent");

        // Mark state as killed.
        let now = crate::run::now_rfc3339_pub();
        let new_state = JobState {
            job: JobStateJob {
                id: opts.job_id.to_string(),
                status: JobStatus::Killed,
                started_at: state.started_at().to_string(),
            },
            result: JobStateResult {
                exit_code: None,
                signal: Some(signal_upper.clone()),
                duration_ms: None,
            },
            pid: Some(pid),
            finished_at: Some(now.clone()),
            updated_at: now,
        };
        job_dir.write_state(&new_state)?;
    }

    let response = Response::new(
        "kill",
        KillData {
            job_id: opts.job_id.to_string(),
            signal: signal_upper,
        },
    );
    response.print();
    Ok(())
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: &str) -> Result<()> {
    let signum: libc::c_int = match signal {
        "TERM" => libc::SIGTERM,
        "INT" => libc::SIGINT,
        "KILL" => libc::SIGKILL,
        _ => libc::SIGKILL, // Unknown → KILL
    };
    // SAFETY: kill(2) is safe to call with any pid and valid signal number.
    let ret = unsafe { libc::kill(pid as libc::pid_t, signum) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        // ESRCH (3): No such process — already gone, treat as success.
        if err.raw_os_error() != Some(libc::ESRCH) {
            return Err(err.into());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn send_signal(pid: u32, _signal: &str) -> Result<()> {
    // On Windows, use a Job Object to terminate the entire process tree.
    // This is equivalent to POSIX process-group kill and satisfies the
    // "process tree termination" requirement on Windows.
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    unsafe {
        // Open the target process.
        let proc_handle: HANDLE = OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, false, pid)?;

        // Create a new Job Object and assign the process to it, then terminate
        // all processes in the job (the target process and any children it
        // has already spawned).
        let job: HANDLE = CreateJobObjectW(None, None)?;

        // Assign process to the job (if it is already in a job this may fail,
        // e.g. when the process is already a member of another job object).
        // In either case, we must guarantee process-tree termination per spec.
        if AssignProcessToJobObject(job, proc_handle).is_err() {
            // The process belongs to an existing job object (common when the
            // supervisor itself runs inside a job, e.g. CI environments).
            // Fall back to recursive tree termination via snapshot enumeration
            // so that child processes are also killed, fulfilling the MUST
            // requirement from spec.md:55-63.
            let _ = CloseHandle(job);
            let _ = CloseHandle(proc_handle);
            // Propagate error if tree termination fails — success must not be
            // returned unless the entire process tree is actually terminated.
            return terminate_process_tree(pid);
        }

        // Terminate all processes in the job (process tree).
        // Per spec.md:55-63, failure here must be surfaced as an error because
        // the caller cannot verify tree termination otherwise.
        TerminateJobObject(job, 1).map_err(|e| {
            let _ = CloseHandle(proc_handle);
            let _ = CloseHandle(job);
            anyhow::anyhow!("TerminateJobObject failed: {}", e)
        })?;

        let _ = CloseHandle(proc_handle);
        let _ = CloseHandle(job);
    }
    Ok(())
}

/// Recursively terminate a process and all its descendants using
/// CreateToolhelp32Snapshot. This is the fallback path when Job Object
/// assignment fails (e.g., nested job objects on older Windows or CI).
///
/// Returns `Ok(())` only when the entire process tree (root + all descendants)
/// has been terminated. Returns an error if snapshot enumeration fails or if
/// the root process itself cannot be opened for termination, because in those
/// cases tree-wide termination cannot be guaranteed (spec.md:55-63 MUST).
#[cfg(windows)]
fn terminate_process_tree(root_pid: u32) -> Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        // Build a list of (pid, parent_pid) for all running processes.
        // If we cannot take a snapshot we cannot enumerate child processes, so
        // we must return an error rather than silently skip them.
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| anyhow::anyhow!("CreateToolhelp32Snapshot failed: {}", e))?;

        let mut entries: Vec<(u32, u32)> = Vec::new();
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };

        if Process32First(snapshot, &mut entry).is_ok() {
            loop {
                entries.push((entry.th32ProcessID, entry.th32ParentProcessID));
                entry = PROCESSENTRY32 {
                    dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
                    ..Default::default()
                };
                if Process32Next(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);

        // Collect all pids in the subtree rooted at root_pid (BFS).
        let mut to_kill: Vec<u32> = vec![root_pid];
        let mut i = 0;
        while i < to_kill.len() {
            let parent = to_kill[i];
            for &(child_pid, parent_pid) in &entries {
                if parent_pid == parent && !to_kill.contains(&child_pid) {
                    to_kill.push(child_pid);
                }
            }
            i += 1;
        }

        // Terminate all collected processes (children first, then root).
        // Per spec.md:55-63, tree-wide termination is a MUST.  Every process
        // in the subtree must be confirmed terminated; failure to terminate
        // any process (root or child) returns an error unless the process no
        // longer exists (already terminated, which is a success condition).
        use windows::Win32::Foundation::ERROR_INVALID_PARAMETER;

        for &target_pid in to_kill.iter().rev() {
            match OpenProcess(PROCESS_TERMINATE, false, target_pid) {
                Ok(h) => {
                    let result = TerminateProcess(h, 1);
                    let _ = CloseHandle(h);
                    result.map_err(|e| {
                        anyhow::anyhow!("TerminateProcess for pid {} failed: {}", target_pid, e)
                    })?;
                }
                Err(e) => {
                    // ERROR_INVALID_PARAMETER (87) means the process no longer
                    // exists — it has already exited, which is a success
                    // condition (the process is gone).  Any other error means
                    // we could not open the process handle and therefore cannot
                    // confirm or perform termination, which violates the MUST.
                    if e.code() != ERROR_INVALID_PARAMETER.to_hresult() {
                        return Err(anyhow::anyhow!(
                            "OpenProcess for pid {} failed (process may still be running): {}",
                            target_pid,
                            e
                        ));
                    }
                    // Process already gone — treat as success.
                }
            }
        }
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn send_signal(_pid: u32, _signal: &str) -> Result<()> {
    anyhow::bail!("kill not supported on this platform");
}
