//! Implementation of the `status` sub-command.
//!
//! `status` is a read-only query: it reads `meta.json`, `state.json`, and the
//! size metadata of the canonical log files, and never writes back.  In
//! particular a persisted `running` record whose PID is gone keeps
//! `state = "running"` and is distinguished by `process_alive = false`; `list`
//! presents the same record as `unknown`.
//!
//! Log totals come from `std::fs::metadata`, so the response cost does not grow
//! with log size and log contents are never read.  Use `tail` for content.

use anyhow::Result;
use tracing::debug;

use crate::jobstore::{JobDir, resolve_root};
use crate::process::pid_liveness;
use crate::run::{now_rfc3339_pub, parse_rfc3339_secs};
use crate::schema::{JobStatus, Response, StatusData};

/// Options for the `status` sub-command.
#[derive(Debug)]
pub struct StatusOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
}

/// Execute `status`: read job state and emit JSON.
pub fn execute(opts: StatusOpts) -> Result<()> {
    status_response(opts)?.print();
    Ok(())
}

/// Bounded size observation for one canonical log file.
///
/// A missing or unreadable log reports `0` rather than failing the query.
fn observed_bytes(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Best-effort liveness for a persisted `running` record.
///
/// The probe runs only for `running` state because a recorded PID may have been
/// reused once the job is terminal.  `None` means no live observation was made
/// and the field is omitted from the response.
fn resolve_process_alive(status: &JobStatus, pid: Option<u32>) -> Option<bool> {
    if *status != JobStatus::Running {
        return None;
    }
    match pid {
        Some(pid) => pid_liveness(pid),
        // A running record without a PID cannot have a live observed process.
        None => Some(false),
    }
}

/// Live elapsed time for a non-terminal job that has started.
///
/// Terminal jobs report the persisted `duration_ms` instead; a null persisted
/// duration is never recomputed at read time.
fn resolve_elapsed_ms(status: &JobStatus, started_at: Option<&str>, now: &str) -> Option<u64> {
    if !status.is_non_terminal() {
        return None;
    }
    let started_secs = parse_rfc3339_secs(started_at?)?;
    let now_secs = parse_rfc3339_secs(now)?;
    Some(now_secs.saturating_sub(started_secs) * 1000)
}

pub fn status_response(opts: StatusOpts) -> Result<Response<StatusData>> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let meta = job_dir.read_meta()?;
    let state = job_dir.read_state()?;
    let status = state.status().clone();

    debug!(job_id = %opts.job_id, state = ?status, "status query");

    let process_alive = resolve_process_alive(&status, state.pid);
    let elapsed_ms = resolve_elapsed_ms(&status, state.started_at(), &now_rfc3339_pub());

    let stdout_log_path = job_dir.stdout_path();
    let stderr_log_path = job_dir.stderr_path();

    // Read-only: a job whose supervisor never authored a control record reports
    // null abandonment fields rather than having one materialized for it.
    let abandon = crate::abandon::status_for(&job_dir, &status);

    let response = Response::new(
        "status",
        StatusData {
            job_id: job_dir.job_id.clone(),
            state: status.as_str().to_string(),
            exit_code: state.exit_code(),
            created_at: meta.created_at,
            started_at: state.started_at().map(|s| s.to_string()),
            finished_at: state.finished_at.clone(),
            command: meta.command,
            cwd: meta.cwd,
            tags: meta.tags,
            pid: state.pid,
            process_alive,
            updated_at: state.updated_at.clone(),
            elapsed_ms,
            duration_ms: state.duration_ms(),
            signal: state.signal().map(|s| s.to_string()),
            logs_drained: state.logs_drained,
            // Projected verbatim from persisted state: `status` never synthesizes
            // abandonment provenance for records that do not carry it.
            abandoned_by: state.abandoned_by.clone(),
            result_loss: state.result_loss,
            stdout_total_bytes: observed_bytes(&stdout_log_path),
            stderr_total_bytes: observed_bytes(&stderr_log_path),
            stdout_log_path: stdout_log_path.display().to_string(),
            stderr_log_path: stderr_log_path.display().to_string(),
            abandon_job_after_ms: abandon.abandon_job_after_ms,
            abandon_deadline: abandon.abandon_deadline,
            abandon_remaining_ms: abandon.abandon_remaining_ms,
            abandon_revision: abandon.abandon_revision,
            abandon_configured_by: abandon.abandon_configured_by,
        },
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_alive_is_omitted_for_terminal_state() {
        for status in [JobStatus::Exited, JobStatus::Killed, JobStatus::Failed] {
            assert_eq!(resolve_process_alive(&status, Some(1)), None);
        }
    }

    #[test]
    fn process_alive_is_omitted_for_created_state() {
        assert_eq!(resolve_process_alive(&JobStatus::Created, Some(1)), None);
    }

    #[test]
    fn process_alive_is_false_when_running_record_has_no_pid() {
        assert_eq!(
            resolve_process_alive(&JobStatus::Running, None),
            Some(false)
        );
    }

    #[test]
    fn elapsed_ms_is_response_time_minus_started_at() {
        assert_eq!(
            resolve_elapsed_ms(
                &JobStatus::Running,
                Some("2024-01-01T00:00:00Z"),
                "2024-01-01T00:00:07Z"
            ),
            Some(7000)
        );
    }

    #[test]
    fn elapsed_ms_is_omitted_for_terminal_state() {
        assert_eq!(
            resolve_elapsed_ms(
                &JobStatus::Exited,
                Some("2024-01-01T00:00:00Z"),
                "2024-01-01T00:00:07Z"
            ),
            None
        );
    }

    #[test]
    fn elapsed_ms_is_omitted_without_started_at() {
        assert_eq!(
            resolve_elapsed_ms(&JobStatus::Created, None, "2024-01-01T00:00:07Z"),
            None
        );
    }

    #[test]
    fn elapsed_ms_never_goes_negative_for_clock_skew() {
        assert_eq!(
            resolve_elapsed_ms(
                &JobStatus::Running,
                Some("2024-01-01T00:00:07Z"),
                "2024-01-01T00:00:00Z"
            ),
            Some(0)
        );
    }
}
