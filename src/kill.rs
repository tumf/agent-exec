//! Implementation of the `kill` sub-command.
//!
//! Signals supported: TERM, INT, KILL (case-insensitive).
//!
//! Signal mapping on Windows:
//!   TERM → TerminateJobObject (graceful intent; Windows has no SIGTERM, so
//!           tree termination is the closest equivalent)
//!   INT  → TerminateJobObject (same; Windows has no SIGINT for arbitrary PIDs)
//!   KILL → TerminateJobObject (forced; semantically the same on Windows)
//!   *    → TerminateJobObject (unknown signals treated as KILL per design.md)
//!
//! On Windows the supervisor records a `windows_job_name` in `state.json`.
//! When present, `kill` opens that named Job Object directly and terminates
//! it, which stops the entire process tree.  If absent (e.g. the supervisor
//! could not assign the process to a job), a snapshot-based tree enumeration
//! fallback is used instead.

use anyhow::Result;
use tracing::info;

use crate::jobstore::{InvalidJobState, JobDir, resolve_root};
use crate::schema::{JobState, JobStateJob, JobStateResult, JobStatus, KillData, Response};

/// Options for the `kill` sub-command.
#[derive(Debug)]
pub struct KillOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// Signal name: TERM | INT | KILL (default: TERM).
    pub signal: &'a str,
    /// Skip post-signal observation and return immediately (legacy shape).
    pub no_wait: bool,
}

impl<'a> Default for KillOpts<'a> {
    fn default() -> Self {
        KillOpts {
            job_id: "",
            root: None,
            signal: "TERM",
            no_wait: false,
        }
    }
}

/// Execute `kill`: send signal, optionally observe post-signal state, and emit JSON.
pub fn execute(opts: KillOpts) -> Result<()> {
    kill_response(opts)?.print();
    Ok(())
}

pub fn kill_response(opts: KillOpts) -> Result<Response<KillData>> {
    Ok(Response::new("kill", execute_inner(opts)?))
}

/// Core kill logic returning `KillData`. Shared by CLI and HTTP handler.
pub fn execute_inner(opts: KillOpts) -> Result<KillData> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let state = job_dir.read_state()?;
    let signal_upper = opts.signal.to_uppercase();

    if *state.status() == JobStatus::Created {
        return Err(anyhow::Error::new(InvalidJobState(format!(
            "job {} is in 'created' state and has not been started; cannot send signal",
            opts.job_id
        ))));
    }

    if *state.status() != JobStatus::Running {
        return Ok(KillData {
            job_id: job_dir.job_id.clone(),
            signal: signal_upper,
            state: if opts.no_wait {
                None
            } else {
                Some(state.status().as_str().to_string())
            },
            exit_code: if opts.no_wait {
                None
            } else {
                state.exit_code()
            },
            terminated_signal: if opts.no_wait {
                None
            } else {
                state.result.signal.clone()
            },
            observed_within_ms: if opts.no_wait { None } else { Some(0) },
        });
    }

    if let Some(pid) = state.pid {
        #[cfg(windows)]
        send_signal(pid, &signal_upper, state.windows_job_name.as_deref())?;
        #[cfg(not(windows))]
        send_signal(pid, &signal_upper)?;

        info!(job_id = %job_dir.job_id, pid, signal = %signal_upper, "signal sent");

        let now = crate::run::now_rfc3339_pub();
        let new_state = JobState {
            job: JobStateJob {
                id: job_dir.job_id.clone(),
                status: JobStatus::Killed,
                started_at: state.started_at().map(|s| s.to_string()),
            },
            result: JobStateResult {
                exit_code: None,
                signal: Some(signal_upper.clone()),
                duration_ms: None,
            },
            pid: Some(pid),
            finished_at: Some(now.clone()),
            updated_at: now,
            logs_drained: true,
            windows_job_name: None,
        };
        job_dir.write_state(&new_state)?;
    }

    if opts.no_wait {
        return Ok(KillData {
            job_id: job_dir.job_id.clone(),
            signal: signal_upper,
            state: None,
            exit_code: None,
            terminated_signal: None,
            observed_within_ms: None,
        });
    }

    let obs = observe_post_signal(&job_dir, std::time::Duration::from_secs(3));

    Ok(KillData {
        job_id: job_dir.job_id.clone(),
        signal: signal_upper,
        state: Some(obs.state),
        exit_code: obs.exit_code,
        terminated_signal: obs.terminated_signal,
        observed_within_ms: Some(obs.observed_within_ms),
    })
}

struct PostSignalObservation {
    state: String,
    exit_code: Option<i32>,
    terminated_signal: Option<String>,
    observed_within_ms: u64,
}

fn observe_post_signal(job_dir: &JobDir, budget: std::time::Duration) -> PostSignalObservation {
    let start = std::time::Instant::now();
    let deadline = start + budget;
    let poll_interval = std::time::Duration::from_millis(100);

    loop {
        if let Ok(st) = job_dir.read_state()
            && !st.status().is_non_terminal()
        {
            return PostSignalObservation {
                state: st.status().as_str().to_string(),
                exit_code: st.exit_code(),
                terminated_signal: st.result.signal.clone(),
                observed_within_ms: start.elapsed().as_millis() as u64,
            };
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(poll_interval);
    }

    if let Ok(st) = job_dir.read_state() {
        PostSignalObservation {
            state: st.status().as_str().to_string(),
            exit_code: st.exit_code(),
            terminated_signal: st.result.signal.clone(),
            observed_within_ms: start.elapsed().as_millis() as u64,
        }
    } else {
        PostSignalObservation {
            state: "running".to_string(),
            exit_code: None,
            terminated_signal: None,
            observed_within_ms: start.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: &str) -> Result<()> {
    let signum: libc::c_int = match signal {
        "TERM" => libc::SIGTERM,
        "INT" => libc::SIGINT,
        "KILL" => libc::SIGKILL,
        _ => libc::SIGKILL, // Unknown → KILL (per design.md)
    };
    // Send signal to the process group (negative PID) so the shell wrapper
    // and all its descendants receive it.  Fall back to single-process kill
    // if the process-group kill fails (e.g. process is not a group leader).
    // SAFETY: kill(2) is safe to call with any pid and valid signal number.
    let pgid = -(pid as libc::pid_t);
    let ret = unsafe { libc::kill(pgid, signum) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            // No such process group — try single-process kill as fallback.
            let ret2 = unsafe { libc::kill(pid as libc::pid_t, signum) };
            if ret2 != 0 {
                let err2 = std::io::Error::last_os_error();
                if err2.raw_os_error() != Some(libc::ESRCH) {
                    return Err(err2.into());
                }
            }
        } else {
            return Err(err.into());
        }
    }
    Ok(())
}

/// Windows signal dispatch.
///
/// Signal mapping (per design.md):
/// - TERM/INT/KILL all map to Job Object termination (process tree termination).
/// - Unknown signals are treated as KILL (same as design.md specifies).
///
/// Strategy:
/// 1. If `job_name` is Some, open the named Job Object and call TerminateJobObject.
/// 2. Otherwise fall back to snapshot-based tree enumeration starting at `pid`.
#[cfg(windows)]
fn send_signal(pid: u32, signal: &str, job_name: Option<&str>) -> Result<()> {
    use tracing::debug;
    use windows::Win32::Foundation::CloseHandle;

    // Log the signal mapping for observability.
    let _mapped = match signal {
        "TERM" => "TerminateJobObject (TERM→process-tree kill)",
        "INT" => "TerminateJobObject (INT→process-tree kill)",
        "KILL" => "TerminateJobObject (KILL→process-tree kill)",
        other => {
            debug!(
                signal = other,
                "unknown signal mapped to KILL (process-tree kill)"
            );
            "TerminateJobObject (unknown→process-tree kill)"
        }
    };

    // Path 1: named Job Object created by the supervisor is available.
    if let Some(name) = job_name {
        use windows::Win32::System::JobObjects::{
            JOB_OBJECT_ALL_ACCESS, OpenJobObjectW, TerminateJobObject,
        };
        use windows::core::HSTRING;

        let hname = HSTRING::from(name);
        unsafe {
            let job = OpenJobObjectW(JOB_OBJECT_ALL_ACCESS, false, &hname)
                .map_err(|e| anyhow::anyhow!("OpenJobObjectW({name}) failed: {e}"))?;
            let result = TerminateJobObject(job, 1)
                .map_err(|e| anyhow::anyhow!("TerminateJobObject({name}) failed: {e}"));
            let _ = CloseHandle(job);
            return result;
        }
    }

    // Path 2: no named Job Object — try ad-hoc assignment then terminate.
    send_signal_no_job(pid)
}

/// Fallback Windows kill path when no named Job Object is available.
/// Attempts to create a temporary Job Object, assign the process, and terminate.
/// If assignment fails (process already in another job), falls back to
/// snapshot-based recursive tree termination.
#[cfg(windows)]
fn send_signal_no_job(pid: u32) -> Result<()> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    unsafe {
        // Open the target process.
        let proc_handle: HANDLE = OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, false, pid)?;

        // Create an anonymous Job Object and assign the process to it, then
        // terminate all processes in the job (the target process and any
        // children it has already spawned).
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
        CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

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
