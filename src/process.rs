//! Shared best-effort process observation.
//!
//! `list`, `delete`, and `status` all need to know whether a persisted PID still
//! looks alive.  The probe is deliberately tri-state so each caller can decide
//! what "no observation was possible" means for its own contract:
//!
//! - `Some(true)`  — the process appears to exist (possibly owned by another user).
//! - `Some(false)` — the process appears absent or dead.
//! - `None`        — this platform provides no probe, so nothing was observed.
//!
//! The observation is best effort and same-user scoped.  It is not an
//! authoritative liveness guarantee: PIDs may be reused, and platform probes may
//! classify inaccessible processes differently.

/// Best-effort liveness probe for a persisted PID.
#[cfg(unix)]
pub fn pid_liveness(pid: u32) -> Option<bool> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if ret == 0 {
        return Some(true);
    }

    // EPERM means the process exists but belongs to another user.
    let err = std::io::Error::last_os_error();
    Some(matches!(err.raw_os_error(), Some(libc::EPERM)))
}

/// Best-effort liveness probe for a persisted PID.
#[cfg(windows)]
pub fn pid_liveness(pid: u32) -> Option<bool> {
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
        Ok(handle) => handle,
        Err(_) => return Some(false),
    };

    let mut exit_code = 0u32;
    let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) }.is_ok();
    unsafe {
        let _ = CloseHandle(handle);
    }
    Some(ok && exit_code == STILL_ACTIVE.0)
}

/// Best-effort liveness probe for a persisted PID.
///
/// No probe exists on this platform, so no observation is made.
#[cfg(not(any(unix, windows)))]
pub fn pid_liveness(_pid: u32) -> Option<bool> {
    None
}
