//! Revisioned job-local abandonment control.
//!
//! A launch-time `abandon-job-after` limit is only the *first* value of a
//! control that stays mutable while the job runs. This module owns the durable
//! record that both the updater (`abandon set` / `abandon clear` and every
//! equivalent adapter) and the supervisor read and replace, plus the advisory
//! lock that makes those two writers linearizable.
//!
//! # Why a separate record
//!
//! `meta.json` describes the *definition* a job was launched with. Editing it
//! cannot make a claim about the detached supervisor's runtime timing, so a
//! metadata-only edit could report success while a stale deadline was already
//! terminating the workload. The control record is authored by the supervisor
//! itself, which is what makes "the supervisor will honour this" a checkable
//! statement rather than a hope: a running job without a supervisor-authored
//! record is controlled by an incompatible legacy supervisor and rejects
//! updates instead of lying about them.
//!
//! # Linearization
//!
//! Updater and supervisor lock the same fixed-name per-job file
//! ([`LOCK_FILE`]). The lock is deliberately *not* placed on the control JSON
//! itself: that file is replaced by atomic rename, so its inode changes and a
//! lock held on it would protect nothing. Advisory locks are released by the
//! operating system when the holding process dies, so a crash cannot wedge the
//! job.
//!
//! Two linearization points exist:
//!
//! 1. the atomic replacement of the control record by an accepted set/clear;
//! 2. the atomic transition of the current revision to
//!    [`AbandonPhase::Triggered`], which the supervisor performs *before* it
//!    signals.
//!
//! Whichever commits first wins: an update that lands first is observed by the
//! supervisor before it signals, and a trigger that lands first makes every
//! later update fail with `invalid_state` rather than claim it prevented an
//! abandonment that had already begun.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::jobstore::{InvalidJobState, JobDir, resolve_root, write_atomic};
use crate::schema::{AbandonData, JobStatus};

/// Fixed name of the job-local control record.
pub const CONTROL_FILE: &str = "abandon_control.json";

/// Fixed name of the per-job advisory lock file.
///
/// Never the control record itself: the record is replaced by rename, so a lock
/// on its inode would be dropped by the very write it is supposed to guard.
pub const LOCK_FILE: &str = "abandon.lock";

/// Lifecycle phase of the current control revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbandonPhase {
    /// A deadline is armed and the supervisor will act on it.
    Active,
    /// No deadline is armed; the workload runs without a destructive limit.
    Disabled,
    /// The supervisor committed this revision to abandonment before signaling.
    Triggered,
}

impl AbandonPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            AbandonPhase::Active => "active",
            AbandonPhase::Disabled => "disabled",
            AbandonPhase::Triggered => "triggered",
        }
    }
}

/// Which surface configured the current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfiguredBy {
    /// The limit came from the job definition at launch time.
    Launch,
    /// The limit came from a runtime set/clear against a running job.
    Update,
}

impl ConfiguredBy {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfiguredBy::Launch => "launch",
            ConfiguredBy::Update => "update",
        }
    }
}

/// The durable runtime source of truth for one job's abandonment control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbandonControl {
    /// Monotonically increasing revision. A signal is only ever committed
    /// against the revision the supervisor re-read under the lock.
    pub revision: u64,
    pub phase: AbandonPhase,
    /// Duration accepted for this revision; `null` when disabled.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub abandon_job_after_ms: Option<u64>,
    /// Absolute RFC 3339 deadline; `null` when disabled.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deadline: Option<String>,
    /// `null` only for an unlimited job that was never configured destructively.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub configured_by: Option<ConfiguredBy>,
    /// Acceptance timestamp of this revision.
    pub updated_at: String,
    /// Workload the triggered transition targets; set only while `triggered`.
    ///
    /// This is what lets a replacement supervisor tell "a crash happened between
    /// persisting the transition and signaling, and the workload is still alive"
    /// apart from "the abandonment already completed".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub triggered_pid: Option<u32>,
}

impl AbandonControl {
    /// Absolute deadline in Unix milliseconds, when one is armed.
    pub fn deadline_ms(&self) -> Option<u64> {
        self.deadline.as_deref().and_then(parse_rfc3339_ms)
    }

    /// Whether an armed deadline has been reached at `now_ms`.
    pub fn is_due(&self, now_ms: u64) -> bool {
        matches!(self.phase, AbandonPhase::Active)
            && self.deadline_ms().is_some_and(|due| now_ms >= due)
    }

    fn to_data(&self, job_id: &str) -> AbandonData {
        AbandonData {
            job_id: job_id.to_string(),
            abandon_revision: self.revision,
            abandon_phase: self.phase.as_str().to_string(),
            abandon_job_after_ms: self.abandon_job_after_ms,
            abandon_deadline: self.deadline.clone(),
            abandon_configured_by: self.configured_by.map(|c| c.as_str().to_string()),
            updated_at: self.updated_at.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// Current wall-clock time in Unix milliseconds.
///
/// Wall clock rather than a monotonic instant because the deadline has to
/// survive a restart into a different process, which no monotonic clock can do.
/// The cost is explicit in the contract: clock movement changes the real elapsed
/// time to a persisted deadline, and `abandon_remaining_ms` is advisory.
pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// Format Unix milliseconds as an RFC 3339 timestamp with millisecond precision.
pub fn format_rfc3339_ms(unix_ms: u64) -> String {
    let secs = unix_ms / 1000;
    let millis = unix_ms % 1000;
    let base = crate::run::format_rfc3339_pub(secs);
    let stem = base.strip_suffix('Z').unwrap_or(&base);
    format!("{stem}.{millis:03}Z")
}

/// Parse an RFC 3339 timestamp into Unix milliseconds.
///
/// Reuses the crate's whole-second parser and adds the optional fractional part
/// so a deadline written by this module round-trips without losing sub-second
/// resolution.
pub fn parse_rfc3339_ms(value: &str) -> Option<u64> {
    let secs = crate::run::parse_rfc3339_secs(value)?;
    let millis = fractional_millis(value);
    secs.checked_mul(1000)?.checked_add(millis)
}

/// Millisecond component of an RFC 3339 fractional-seconds suffix.
fn fractional_millis(value: &str) -> u64 {
    let rest = match value.get(19..) {
        Some(rest) => rest,
        None => return 0,
    };
    let digits = match rest.strip_prefix(['.', ',']) {
        Some(digits) => digits,
        None => return 0,
    };
    let digits: String = digits.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return 0;
    }
    // Normalize to exactly three digits: "5" is 500ms, "0005" is 0ms.
    let mut normalized: String = digits.chars().take(3).collect();
    while normalized.len() < 3 {
        normalized.push('0');
    }
    normalized.parse().unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Paths, persistence, locking
// ---------------------------------------------------------------------------

pub fn control_path(job_dir: &JobDir) -> std::path::PathBuf {
    job_dir.path.join(CONTROL_FILE)
}

pub fn lock_path(job_dir: &JobDir) -> std::path::PathBuf {
    job_dir.path.join(LOCK_FILE)
}

/// Read the durable control record, or `None` when no compatible supervisor
/// authored one.
///
/// A record that cannot be parsed is reported as absent rather than as an
/// error: the only writer replaces it atomically, so unreadable content means
/// "written by something that is not this contract", which is exactly the
/// legacy-supervisor case.
pub fn read_control(job_dir: &JobDir) -> Option<AbandonControl> {
    let raw = std::fs::read(control_path(job_dir)).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Replace the control record atomically.
///
/// A crash before the rename leaves the previous complete record; a crash after
/// it leaves the new complete record. No partial JSON is ever admitted.
fn write_control(job_dir: &JobDir, control: &AbandonControl) -> Result<()> {
    let target = control_path(job_dir);
    let contents = serde_json::to_string_pretty(control)?;
    write_atomic(&job_dir.path, &target, contents.as_bytes())
}

/// Exclusive advisory lock over one job's abandonment transitions.
///
/// Held by both the updater and the supervisor. Dropping the guard (including
/// when the process dies) releases it.
pub struct ControlLock {
    file: std::fs::File,
}

impl Drop for ControlLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Acquire the fixed-name per-job advisory lock, blocking until it is free.
pub fn lock_control(job_dir: &JobDir) -> Result<ControlLock> {
    let path = lock_path(job_dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open abandonment control lock {}", path.display()))?;
    file.lock()
        .with_context(|| format!("acquire abandonment control lock {}", path.display()))?;
    Ok(ControlLock { file })
}

// ---------------------------------------------------------------------------
// Supervisor-side transitions
// ---------------------------------------------------------------------------

/// Build the control a launch or relaunch arms from a configured duration.
///
/// Kept free of I/O so the launch/restart decision is directly unit-testable.
pub fn control_from_duration(
    revision: u64,
    abandon_job_after_ms: u64,
    now_ms: u64,
    configured_by: ConfiguredBy,
) -> AbandonControl {
    if abandon_job_after_ms == 0 {
        return AbandonControl {
            revision,
            phase: AbandonPhase::Disabled,
            abandon_job_after_ms: None,
            deadline: None,
            // A job that was never configured destructively has no source to
            // report; an explicit clear does, and passes `Update` here.
            configured_by: match configured_by {
                ConfiguredBy::Launch => None,
                ConfiguredBy::Update => Some(ConfiguredBy::Update),
            },
            updated_at: format_rfc3339_ms(now_ms),
            triggered_pid: None,
        };
    }
    AbandonControl {
        revision,
        phase: AbandonPhase::Active,
        abandon_job_after_ms: Some(abandon_job_after_ms),
        deadline: Some(format_rfc3339_ms(
            now_ms.saturating_add(abandon_job_after_ms),
        )),
        configured_by: Some(configured_by),
        updated_at: format_rfc3339_ms(now_ms),
        triggered_pid: None,
    }
}

/// Decide the control a starting supervisor must arm.
///
/// The launch-time definition is authoritative only for `launch`-configured
/// controls. A control the operator changed at runtime keeps its accepted
/// absolute deadline and revision across a restart, which is what stops a
/// restart from silently restoring the deadline the operator just replaced.
/// A record left `triggered` by a previous run is re-armed rather than replayed:
/// resuming a persisted transition belongs to the recovery path, which knows
/// whether the workload it targeted is still alive.
pub fn plan_launch_control(
    existing: Option<&AbandonControl>,
    launch_abandon_job_after_ms: u64,
    now_ms: u64,
) -> Option<AbandonControl> {
    let Some(current) = existing else {
        return Some(control_from_duration(
            1,
            launch_abandon_job_after_ms,
            now_ms,
            ConfiguredBy::Launch,
        ));
    };

    if current.configured_by == Some(ConfiguredBy::Update) {
        return match current.phase {
            // The accepted deadline (or the accepted clear) survives verbatim.
            AbandonPhase::Active | AbandonPhase::Disabled => None,
            // The transition already committed against a workload that is gone;
            // the new run inherits the same due deadline and fires immediately.
            AbandonPhase::Triggered => Some(AbandonControl {
                revision: current.revision.saturating_add(1),
                phase: AbandonPhase::Active,
                updated_at: format_rfc3339_ms(now_ms),
                triggered_pid: None,
                ..current.clone()
            }),
        };
    }

    Some(control_from_duration(
        current.revision.saturating_add(1),
        launch_abandon_job_after_ms,
        now_ms,
        ConfiguredBy::Launch,
    ))
}

/// Author or re-arm this job's control record before the workload launches.
///
/// Publishing the record before the supervisor's `running` transition is what
/// makes the compatibility rule observable: a job that reports `running` and has
/// no control record can only have been launched by a supervisor that predates
/// this contract.
pub fn prepare_launch_control(
    job_dir: &JobDir,
    launch_abandon_job_after_ms: u64,
) -> Result<AbandonControl> {
    let _lock = lock_control(job_dir)?;
    let existing = read_control(job_dir);
    match plan_launch_control(
        existing.as_ref(),
        launch_abandon_job_after_ms,
        now_unix_ms(),
    ) {
        Some(control) => {
            write_control(job_dir, &control)?;
            debug!(
                job_id = %job_dir.job_id,
                revision = control.revision,
                phase = control.phase.as_str(),
                "armed abandonment control for launch"
            );
            Ok(control)
        }
        None => {
            let control = existing.expect("preserved control exists");
            debug!(
                job_id = %job_dir.job_id,
                revision = control.revision,
                "preserved update-configured abandonment control across launch"
            );
            Ok(control)
        }
    }
}

/// Result of the supervisor's attempt to commit a due deadline.
#[derive(Debug, PartialEq, Eq)]
pub enum TriggerOutcome {
    /// This revision is now durably `triggered`; the caller must signal.
    Triggered,
    /// The deadline moved out and is no longer due.
    NotDue,
    /// A newer revision, a clear, or another trigger won the lock first.
    Superseded,
}

/// Atomically commit `revision` to `triggered`, or report why it must not fire.
///
/// This is the trigger linearization point: the caller may signal only after
/// this returns [`TriggerOutcome::Triggered`], so a successful update can never
/// be followed by a stale deadline signal.
pub fn try_trigger(
    job_dir: &JobDir,
    revision: u64,
    workload_pid: u32,
    now_ms: u64,
) -> Result<TriggerOutcome> {
    let _lock = lock_control(job_dir)?;
    let Some(current) = read_control(job_dir) else {
        return Ok(TriggerOutcome::Superseded);
    };
    if current.revision != revision || current.phase != AbandonPhase::Active {
        return Ok(TriggerOutcome::Superseded);
    }
    if !current.is_due(now_ms) {
        return Ok(TriggerOutcome::NotDue);
    }
    let triggered = AbandonControl {
        phase: AbandonPhase::Triggered,
        updated_at: format_rfc3339_ms(now_ms),
        triggered_pid: Some(workload_pid),
        ..current
    };
    write_control(job_dir, &triggered)?;
    Ok(TriggerOutcome::Triggered)
}

/// Send the abandonment signal (and escalation) to a workload process group.
///
/// Split out of the supervisor loop so the recovery path signals exactly the way
/// the original transition would have.
pub fn signal_abandonment(job_id: &str, pid: u32, kill_after_ms: u64) {
    info!(
        job_id,
        pid, "abandonment deadline reached, sending SIGTERM to process group"
    );
    #[cfg(unix)]
    {
        // Negative PID targets the whole group; the workload was placed in its
        // own session by the supervisor.
        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGTERM) };
    }
    if kill_after_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(kill_after_ms));
    }
    info!(job_id, pid, "escalating abandonment to SIGKILL");
    #[cfg(unix)]
    {
        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

/// Resume a persisted `triggered` transition whose workload is still alive.
///
/// A supervisor that died between persisting the transition and signaling would
/// otherwise leave a workload that is durably marked for abandonment but that
/// nothing is going to abandon. Returns `true` when a signal was actually
/// resumed.
pub fn recover_triggered_transition(job_dir: &JobDir, kill_after_ms: u64) -> Result<bool> {
    let control = {
        let _lock = lock_control(job_dir)?;
        match read_control(job_dir) {
            Some(control) if control.phase == AbandonPhase::Triggered => control,
            _ => return Ok(false),
        }
    };

    let Some(pid) = control.triggered_pid else {
        return Ok(false);
    };
    if crate::process::pid_liveness(pid) != Some(true) {
        return Ok(false);
    }

    warn!(
        job_id = %job_dir.job_id,
        revision = control.revision,
        pid,
        "resuming a persisted abandonment transition whose workload is still alive"
    );
    signal_abandonment(&job_dir.job_id, pid, kill_after_ms);
    mark_abandoned_terminal_state(job_dir);
    Ok(true)
}

/// Record the abandonment provenance for a workload the recovery path signaled.
///
/// The supervisor that owned the run is gone, so nothing else is going to write
/// a terminal state for it. A state that already reached terminal is left alone.
fn mark_abandoned_terminal_state(job_dir: &JobDir) {
    let Ok(mut state) = job_dir.read_state() else {
        return;
    };
    if !state.status().is_non_terminal() {
        return;
    }
    let now = crate::run::now_rfc3339_pub();
    state.job.status = JobStatus::Killed;
    state.finished_at = Some(now.clone());
    state.updated_at = now;
    state.logs_drained = true;
    state.abandoned_by = Some(crate::schema::ABANDONED_BY_ABANDON_JOB_AFTER.to_string());
    state.result_loss = Some(true);
    if let Err(err) = job_dir.write_state(&state) {
        warn!(
            job_id = %job_dir.job_id,
            error = %err,
            "failed to persist terminal state for a resumed abandonment"
        );
    }
}

// ---------------------------------------------------------------------------
// Updater-side transitions
// ---------------------------------------------------------------------------

/// Guidance returned when a running job predates the mutable control contract.
pub fn legacy_supervisor_message(job_id: &str) -> String {
    format!(
        "job {job_id} is running under a supervisor that predates runtime abandonment updates, \
         so no supervisor-authored abandonment control record exists and no deadline change can \
         be honoured; restart the job with `agent-exec restart {job_id}` to place it under a \
         compatible supervisor"
    )
}

/// Guidance returned when abandonment has already entered its triggered phase.
pub fn already_triggered_message(job_id: &str, revision: u64) -> String {
    format!(
        "job {job_id} already committed abandonment at control revision {revision}; the deadline \
         can no longer be changed"
    )
}

/// Guidance returned for a `set` that carries no result-loss acknowledgement.
pub fn set_acknowledgement_message(abandon_input: &str, acknowledge_input: &str) -> String {
    format!(
        "{} Replacing a running job's deadline with {abandon_input} therefore requires \
         {acknowledge_input}.",
        crate::run::ABANDON_JOB_AFTER_WARNING
    )
}

/// Sentinel error for a runtime deadline change with no acknowledgement.
///
/// Distinct from the launch-time admission error only in the message it carries;
/// both map to the stable `result_loss_not_acknowledged` code.
pub fn require_set_acknowledgement(
    acknowledge_result_loss: bool,
    abandon_input: &str,
    acknowledge_input: &str,
) -> Result<(), crate::run::ResultLossNotAcknowledged> {
    if acknowledge_result_loss {
        return Ok(());
    }
    Err(crate::run::ResultLossNotAcknowledged(
        set_acknowledgement_message(abandon_input, acknowledge_input),
    ))
}

/// Options shared by the runtime set and clear operations.
pub struct AbandonUpdateOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// `Some(ms)` replaces the deadline relative to durable acceptance;
    /// `None` clears it.
    pub abandon_in_ms: Option<u64>,
}

/// Replace or clear a running job's abandonment deadline.
///
/// Admission, the state check, the record replacement, and the compatibility
/// metadata synchronization all happen under one hold of the per-job lock, so a
/// rejected request never leaves a partially applied change behind.
pub fn update_response(
    opts: AbandonUpdateOpts,
) -> Result<crate::schema::Response<crate::schema::AbandonData>> {
    let kind = if opts.abandon_in_ms.is_some() {
        "abandon_set"
    } else {
        "abandon_clear"
    };
    Ok(crate::schema::Response::new(kind, update_data(opts)?))
}

/// Canonical runtime set/clear transition shared by every adapter.
pub fn update_data(opts: AbandonUpdateOpts) -> Result<AbandonData> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let _lock = lock_control(&job_dir)?;

    // Read state under the lock so an admission decision cannot be made against
    // a lifecycle that a concurrent transition has already left.
    let state = job_dir.read_state()?;
    if *state.status() != JobStatus::Running {
        return Err(anyhow::Error::new(InvalidJobState(format!(
            "job {} is in '{}' state; only a running job's abandonment deadline can be changed",
            job_dir.job_id,
            state.status().as_str()
        ))));
    }

    let Some(current) = read_control(&job_dir) else {
        return Err(anyhow::Error::new(InvalidJobState(
            legacy_supervisor_message(&job_dir.job_id),
        )));
    };
    if current.phase == AbandonPhase::Triggered {
        return Err(anyhow::Error::new(InvalidJobState(
            already_triggered_message(&job_dir.job_id, current.revision),
        )));
    }

    // `--in` is relative to durable acceptance, not to the original job start.
    let next = control_from_duration(
        current.revision.saturating_add(1),
        opts.abandon_in_ms.unwrap_or(0),
        now_unix_ms(),
        ConfiguredBy::Update,
    );
    write_control(&job_dir, &next)?;

    // Synchronize the dual-written compatibility fields *after* the linearization
    // point so an older reader can never resurrect a deadline this operation
    // removed. Clear writes zero to both.
    let mut meta = job_dir.read_meta()?;
    meta.set_abandon_job_after_ms(opts.abandon_in_ms.unwrap_or(0));
    job_dir.write_meta_atomic(&meta)?;

    info!(
        job_id = %job_dir.job_id,
        revision = next.revision,
        phase = next.phase.as_str(),
        "accepted runtime abandonment control update"
    );
    Ok(next.to_data(&job_dir.job_id))
}

// ---------------------------------------------------------------------------
// Status projection
// ---------------------------------------------------------------------------

/// Read-only projection of the effective control for `status` responses.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct AbandonStatus {
    pub abandon_job_after_ms: Option<u64>,
    pub abandon_deadline: Option<String>,
    pub abandon_remaining_ms: Option<u64>,
    pub abandon_revision: Option<u64>,
    pub abandon_configured_by: Option<String>,
}

/// Project the durable control into the diagnostic status fields.
///
/// Deliberately pure: `status` is a read-only query and must never materialize
/// a control record for a job whose supervisor never authored one.
pub fn project_status(
    control: Option<&AbandonControl>,
    status: &JobStatus,
    now_ms: u64,
) -> AbandonStatus {
    let Some(control) = control else {
        return AbandonStatus::default();
    };

    let revision = Some(control.revision);
    let configured_by = control.configured_by.map(|c| c.as_str().to_string());

    if control.phase == AbandonPhase::Disabled {
        return AbandonStatus {
            abandon_revision: revision,
            abandon_configured_by: configured_by,
            ..AbandonStatus::default()
        };
    }

    let duration = control.abandon_job_after_ms;
    let deadline_ms = control.deadline_ms();
    // Remaining time is computed at response time and only for a job that can
    // still reach its deadline. It is diagnostic: firing is decided by the
    // locked control transition, never by this value.
    let remaining = match (status.is_non_terminal(), deadline_ms, duration) {
        (true, Some(deadline_ms), Some(duration)) => {
            Some(deadline_ms.saturating_sub(now_ms).min(duration))
        }
        _ => None,
    };

    AbandonStatus {
        abandon_job_after_ms: duration,
        abandon_deadline: control.deadline.clone(),
        abandon_remaining_ms: remaining,
        abandon_revision: revision,
        abandon_configured_by: configured_by,
    }
}

/// Read-only status projection for one job directory.
pub fn status_for(job_dir: &JobDir, status: &JobStatus) -> AbandonStatus {
    project_status(read_control(job_dir).as_ref(), status, now_unix_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control(phase: AbandonPhase, configured_by: Option<ConfiguredBy>) -> AbandonControl {
        AbandonControl {
            revision: 4,
            phase,
            abandon_job_after_ms: Some(30_000),
            deadline: Some(format_rfc3339_ms(1_700_000_030_000)),
            configured_by,
            updated_at: format_rfc3339_ms(1_700_000_000_000),
            triggered_pid: None,
        }
    }

    #[test]
    fn mutable_abandonment_rfc3339_millis_round_trip() {
        for unix_ms in [0, 1_700_000_000_000, 1_700_000_000_001, 1_700_000_000_999] {
            let formatted = format_rfc3339_ms(unix_ms);
            assert_eq!(parse_rfc3339_ms(&formatted), Some(unix_ms), "{formatted}");
        }
    }

    #[test]
    fn mutable_abandonment_whole_second_timestamps_parse_without_a_fraction() {
        assert_eq!(
            parse_rfc3339_ms("2023-11-14T22:13:20Z"),
            Some(1_700_000_000_000)
        );
    }

    #[test]
    fn mutable_abandonment_fractional_millis_normalizes_digit_counts() {
        assert_eq!(fractional_millis("2023-11-14T22:13:20.5Z"), 500);
        assert_eq!(fractional_millis("2023-11-14T22:13:20.05Z"), 50);
        assert_eq!(fractional_millis("2023-11-14T22:13:20.005Z"), 5);
        assert_eq!(fractional_millis("2023-11-14T22:13:20.0005Z"), 0);
        assert_eq!(fractional_millis("2023-11-14T22:13:20Z"), 0);
    }

    #[test]
    fn mutable_abandonment_zero_duration_launch_control_is_disabled_and_unattributed() {
        let control = control_from_duration(1, 0, 1_700_000_000_000, ConfiguredBy::Launch);
        assert_eq!(control.phase, AbandonPhase::Disabled);
        assert_eq!(control.abandon_job_after_ms, None);
        assert_eq!(control.deadline, None);
        assert_eq!(control.configured_by, None);
    }

    #[test]
    fn mutable_abandonment_cleared_control_attributes_the_clear_to_the_update() {
        let control = control_from_duration(9, 0, 1_700_000_000_000, ConfiguredBy::Update);
        assert_eq!(control.phase, AbandonPhase::Disabled);
        assert_eq!(control.configured_by, Some(ConfiguredBy::Update));
    }

    #[test]
    fn mutable_abandonment_duration_control_deadline_is_relative_to_acceptance() {
        let control = control_from_duration(1, 30_000, 1_700_000_000_000, ConfiguredBy::Update);
        assert_eq!(control.phase, AbandonPhase::Active);
        assert_eq!(control.deadline_ms(), Some(1_700_000_030_000));
        assert_eq!(control.abandon_job_after_ms, Some(30_000));
    }

    #[test]
    fn mutable_abandonment_launch_without_a_record_authors_revision_one() {
        let planned = plan_launch_control(None, 5_000, 1_700_000_000_000).expect("authored");
        assert_eq!(planned.revision, 1);
        assert_eq!(planned.configured_by, Some(ConfiguredBy::Launch));
    }

    #[test]
    fn mutable_abandonment_restart_rearms_a_launch_configured_control_from_its_duration() {
        let existing = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        let planned =
            plan_launch_control(Some(&existing), 5_000, 1_700_000_100_000).expect("rearmed");
        assert_eq!(planned.revision, 5);
        assert_eq!(planned.deadline_ms(), Some(1_700_000_105_000));
        assert_eq!(planned.configured_by, Some(ConfiguredBy::Launch));
    }

    #[test]
    fn mutable_abandonment_restart_preserves_an_update_configured_deadline_and_revision() {
        let existing = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        assert!(plan_launch_control(Some(&existing), 5_000, 1_700_000_100_000).is_none());
    }

    #[test]
    fn mutable_abandonment_restart_preserves_an_update_configured_clear() {
        let mut existing = control(AbandonPhase::Disabled, Some(ConfiguredBy::Update));
        existing.abandon_job_after_ms = None;
        existing.deadline = None;
        assert!(plan_launch_control(Some(&existing), 5_000, 1_700_000_100_000).is_none());
    }

    #[test]
    fn mutable_abandonment_restart_rearms_an_update_configured_triggered_control_to_its_due_deadline()
     {
        let existing = control(AbandonPhase::Triggered, Some(ConfiguredBy::Update));
        let planned =
            plan_launch_control(Some(&existing), 5_000, 1_700_000_100_000).expect("rearmed");
        assert_eq!(planned.phase, AbandonPhase::Active);
        assert_eq!(planned.revision, 5);
        // The accepted deadline is retained, so it is already due and the locked
        // active-to-triggered transition happens immediately.
        assert_eq!(planned.deadline_ms(), Some(1_700_000_030_000));
        assert!(planned.is_due(1_700_000_100_000));
    }

    #[test]
    fn mutable_abandonment_restart_rearms_a_launch_configured_triggered_control_from_its_duration()
    {
        let existing = control(AbandonPhase::Triggered, Some(ConfiguredBy::Launch));
        let planned =
            plan_launch_control(Some(&existing), 5_000, 1_700_000_100_000).expect("rearmed");
        assert_eq!(planned.phase, AbandonPhase::Active);
        assert_eq!(planned.deadline_ms(), Some(1_700_000_105_000));
    }

    #[test]
    fn mutable_abandonment_status_projection_is_empty_without_a_control_record() {
        assert_eq!(
            project_status(None, &JobStatus::Running, 1_700_000_000_000),
            AbandonStatus::default()
        );
    }

    #[test]
    fn mutable_abandonment_status_projection_reports_disabled_control_as_null_durations() {
        let mut disabled = control(AbandonPhase::Disabled, Some(ConfiguredBy::Update));
        disabled.abandon_job_after_ms = None;
        disabled.deadline = None;
        let projected = project_status(Some(&disabled), &JobStatus::Running, 1_700_000_000_000);
        assert_eq!(projected.abandon_job_after_ms, None);
        assert_eq!(projected.abandon_deadline, None);
        assert_eq!(projected.abandon_remaining_ms, None);
        assert_eq!(projected.abandon_revision, Some(4));
        assert_eq!(projected.abandon_configured_by.as_deref(), Some("update"));
    }

    #[test]
    fn mutable_abandonment_status_projection_clamps_remaining_into_the_configured_range() {
        let active = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        // Mid-flight.
        assert_eq!(
            project_status(Some(&active), &JobStatus::Running, 1_700_000_010_000)
                .abandon_remaining_ms,
            Some(20_000)
        );
        // Past the deadline clamps to zero rather than wrapping.
        assert_eq!(
            project_status(Some(&active), &JobStatus::Running, 1_700_000_099_000)
                .abandon_remaining_ms,
            Some(0)
        );
        // A backward clock jump clamps to the configured duration.
        assert_eq!(
            project_status(Some(&active), &JobStatus::Running, 1_600_000_000_000)
                .abandon_remaining_ms,
            Some(30_000)
        );
    }

    #[test]
    fn mutable_abandonment_status_projection_omits_remaining_for_terminal_jobs() {
        let active = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        for status in [JobStatus::Exited, JobStatus::Killed, JobStatus::Failed] {
            let projected = project_status(Some(&active), &status, 1_700_000_010_000);
            assert_eq!(projected.abandon_remaining_ms, None, "{status:?}");
            assert_eq!(projected.abandon_revision, Some(4));
        }
    }

    #[test]
    fn mutable_abandonment_due_deadlines_are_recognized_only_while_active() {
        let active = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        assert!(!active.is_due(1_700_000_029_999));
        assert!(active.is_due(1_700_000_030_000));

        let triggered = control(AbandonPhase::Triggered, Some(ConfiguredBy::Launch));
        assert!(!triggered.is_due(1_700_000_099_000));
    }

    #[test]
    fn mutable_abandonment_set_acknowledgement_is_required_and_carries_the_warning() {
        assert!(require_set_acknowledgement(true, "--in", "--acknowledge-result-loss").is_ok());
        let err = require_set_acknowledgement(false, "--in", "--acknowledge-result-loss")
            .expect_err("unacknowledged set must be rejected");
        assert!(err.to_string().contains("--acknowledge-result-loss"));
        assert!(err.to_string().contains("permanently lose"));
    }

    /// A job directory holding only what the control transitions need.
    fn control_fixture(control: &AbandonControl) -> (tempfile::TempDir, JobDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let job_dir = JobDir {
            path: tmp.path().join("job"),
            job_id: "job".to_string(),
        };
        std::fs::create_dir_all(&job_dir.path).expect("create job dir");
        write_control(&job_dir, control).expect("write control");
        (tmp, job_dir)
    }

    /// The only outcome that permits a signal: the revision the supervisor
    /// re-read under the lock is still the current one and is still due.
    #[test]
    fn mutable_abandonment_trigger_commits_a_due_current_revision() {
        let due = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        let (_tmp, job_dir) = control_fixture(&due);

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("trigger"),
            TriggerOutcome::Triggered
        );
        let committed = read_control(&job_dir).expect("committed control");
        assert_eq!(committed.phase, AbandonPhase::Triggered);
        assert_eq!(committed.revision, 4, "a trigger marks the same revision");
        assert_eq!(committed.triggered_pid, Some(4321));
    }

    /// Update wins: an accepted update replaced the revision between the
    /// observer's read and its locked re-read, so the stale deadline must not
    /// signal.
    #[test]
    fn mutable_abandonment_trigger_is_superseded_by_a_newer_revision() {
        let mut updated = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        updated.revision = 5;
        updated.deadline = Some(format_rfc3339_ms(1_700_000_600_000));
        let (_tmp, job_dir) = control_fixture(&updated);

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("trigger"),
            TriggerOutcome::Superseded
        );
        assert_eq!(
            read_control(&job_dir).expect("control").phase,
            AbandonPhase::Active,
            "a superseded trigger must not commit anything"
        );
    }

    /// Clear wins: the revision the observer saw is gone, and a disabled control
    /// can never be triggered.
    #[test]
    fn mutable_abandonment_trigger_is_superseded_by_a_clear() {
        let mut cleared = control(AbandonPhase::Disabled, Some(ConfiguredBy::Update));
        cleared.abandon_job_after_ms = None;
        cleared.deadline = None;
        let (_tmp, job_dir) = control_fixture(&cleared);

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("trigger"),
            TriggerOutcome::Superseded
        );
    }

    /// Trigger wins against itself: a transition is committed exactly once.
    #[test]
    fn mutable_abandonment_trigger_is_superseded_by_an_earlier_trigger() {
        let due = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        let (_tmp, job_dir) = control_fixture(&due);

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("first trigger"),
            TriggerOutcome::Triggered
        );
        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("second trigger"),
            TriggerOutcome::Superseded
        );
    }

    /// A deadline pushed into the future by an update at the same revision is
    /// simply not due; nothing is committed and the observer keeps waiting.
    #[test]
    fn mutable_abandonment_trigger_reports_a_deadline_that_moved_out_as_not_due() {
        let future = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        let (_tmp, job_dir) = control_fixture(&future);

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_000_000).expect("trigger"),
            TriggerOutcome::NotDue
        );
        assert_eq!(
            read_control(&job_dir).expect("control").phase,
            AbandonPhase::Active
        );
    }

    /// A record that vanished is never treated as permission to signal.
    #[test]
    fn mutable_abandonment_trigger_is_superseded_without_a_control_record() {
        let due = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        let (_tmp, job_dir) = control_fixture(&due);
        std::fs::remove_file(control_path(&job_dir)).expect("remove control");

        assert_eq!(
            try_trigger(&job_dir, 4, 4321, 1_700_000_099_000).expect("trigger"),
            TriggerOutcome::Superseded
        );
    }

    /// Partial or foreign content is reported as "no compatible record" rather
    /// than as a control this contract may act on.
    #[test]
    fn mutable_abandonment_unparseable_control_reads_as_absent() {
        let due = control(AbandonPhase::Active, Some(ConfiguredBy::Launch));
        let (_tmp, job_dir) = control_fixture(&due);
        std::fs::write(control_path(&job_dir), b"{\"revision\": ").expect("truncate control");

        assert!(read_control(&job_dir).is_none());
    }

    #[test]
    fn mutable_abandonment_control_record_round_trips_through_json() {
        let control = control(AbandonPhase::Active, Some(ConfiguredBy::Update));
        let raw = serde_json::to_string(&control).expect("serialize");
        let parsed: AbandonControl = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(parsed.revision, control.revision);
        assert_eq!(parsed.phase, control.phase);
        assert_eq!(parsed.configured_by, control.configured_by);
        assert_eq!(parsed.deadline, control.deadline);
    }
}
