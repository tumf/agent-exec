//! Implementation of the `kill` sub-command.
//!
//! Signals supported: TERM, INT, KILL (case-insensitive).
//! On Windows, all signals map to TerminateProcess (Job Object termination).

use anyhow::Result;
use tracing::info;

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{JobState, JobStatus, KillData, Response};

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

    if state.state != JobStatus::Running {
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
        let new_state = JobState {
            state: JobStatus::Killed,
            pid: Some(pid),
            exit_code: None,
            finished_at: Some(crate::run::now_rfc3339_pub()),
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
            terminate_process_tree(pid);
            return Ok(());
        }

        // Terminate all processes in the job (process tree).
        let _ = TerminateJobObject(job, 1);

        let _ = CloseHandle(proc_handle);
        let _ = CloseHandle(job);
    }
    Ok(())
}

/// Recursively terminate a process and all its descendants using
/// CreateToolhelp32Snapshot. This is the fallback path when Job Object
/// assignment fails (e.g., nested job objects on older Windows or CI).
#[cfg(windows)]
fn terminate_process_tree(root_pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        // Build a list of (pid, parent_pid) for all running processes.
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => {
                // Cannot enumerate processes; terminate root PID directly.
                if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, root_pid) {
                    let _ = TerminateProcess(h, 1);
                    let _ = CloseHandle(h);
                }
                return;
            }
        };

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
        for &target_pid in to_kill.iter().rev() {
            if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, target_pid) {
                let _ = TerminateProcess(h, 1);
                let _ = CloseHandle(h);
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn send_signal(_pid: u32, _signal: &str) -> Result<()> {
    anyhow::bail!("kill not supported on this platform");
}
