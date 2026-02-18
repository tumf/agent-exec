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
    // On Windows, terminate the process (all signals map to terminate).
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)?;
        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);
        result?;
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn send_signal(_pid: u32, _signal: &str) -> Result<()> {
    anyhow::bail!("kill not supported on this platform");
}
