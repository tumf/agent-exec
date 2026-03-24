//! Implementation of the `delete` sub-command.
//!
//! Supports two modes:
//!   - `delete <JOB_ID>`: remove one explicit job directory (non-running only).
//!   - `delete --all [--dry-run]`: remove all terminal jobs whose persisted
//!     `meta.json.cwd` matches the caller's current working directory.
//!
//! `--dry-run` may be combined with either mode to report actions without
//! removing any directories.

use anyhow::{Result, anyhow};
use tracing::debug;

use crate::jobstore::{InvalidJobState, JobDir, resolve_root};
use crate::run::resolve_effective_cwd;
use crate::schema::{DeleteData, DeleteJobResult, JobStatus, Response};

/// Options for the `delete` sub-command.
#[derive(Debug)]
pub struct DeleteOpts<'a> {
    pub root: Option<&'a str>,
    /// When `Some`, delete a single job by ID.  Mutually exclusive with `all`.
    pub job_id: Option<&'a str>,
    /// When true, delete all terminal jobs scoped to the caller's cwd.
    pub all: bool,
    /// When true, report candidates without removing any directories.
    pub dry_run: bool,
}

/// Execute `delete`: dispatch to single-job or bulk mode.
pub fn execute(opts: DeleteOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let root_str = root.display().to_string();

    if let Some(job_id) = opts.job_id {
        delete_single(&root, &root_str, job_id, opts.dry_run)
    } else {
        delete_all(&root, &root_str, opts.dry_run)
    }
}

/// Delete a single explicit job by ID or unambiguous prefix.
///
/// Rejects running jobs with `InvalidJobState`.  Returns `JobNotFound` when
/// the directory does not exist, and `AmbiguousJobId` when the prefix matches
/// multiple jobs.
fn delete_single(
    root: &std::path::Path,
    root_str: &str,
    job_id: &str,
    dry_run: bool,
) -> Result<()> {
    // Use JobDir::open for prefix-based resolution (exact match fast path included).
    // Returns AmbiguousJobId if the prefix matches multiple jobs.
    let job_dir = JobDir::open(root, job_id)?;
    let job_path = job_dir.path;
    // Use the resolved canonical ID in all output (never the user-supplied prefix).
    let resolved_id = job_dir.job_id;

    // Read state to determine whether the job is running.
    let state_path = job_path.join("state.json");
    let state_opt: Option<crate::schema::JobState> = std::fs::read(&state_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());

    let state_str = match &state_opt {
        Some(s) => s.status().as_str().to_string(),
        None => "unknown".to_string(),
    };

    // Reject running jobs.
    if state_opt
        .as_ref()
        .map(|s| *s.status() == JobStatus::Running)
        .unwrap_or(false)
    {
        return Err(anyhow::Error::new(InvalidJobState(format!(
            "cannot delete job {job_id}: job is currently running"
        ))));
    }

    let action = if dry_run {
        debug!(job_id, "delete: dry-run would delete job");
        "would_delete"
    } else {
        std::fs::remove_dir_all(&job_path).map_err(|e| {
            anyhow!(
                "failed to delete job directory {}: {}",
                job_path.display(),
                e
            )
        })?;
        debug!(job_id, "delete: deleted job");
        "deleted"
    };

    Response::new(
        "delete",
        DeleteData {
            root: root_str.to_string(),
            dry_run,
            deleted: if action == "deleted" { 1 } else { 0 },
            skipped: 0,
            jobs: vec![DeleteJobResult {
                job_id: resolved_id,
                state: state_str,
                action: action.to_string(),
                reason: "explicit_delete".to_string(),
            }],
        },
    )
    .print();

    Ok(())
}

/// Delete all terminal jobs whose persisted `meta.json.cwd` matches the
/// caller's current working directory.  Running and created jobs are skipped.
fn delete_all(root: &std::path::Path, root_str: &str, dry_run: bool) -> Result<()> {
    let current_cwd = resolve_effective_cwd(None);

    debug!(
        root = %root_str,
        cwd = %current_cwd,
        dry_run,
        "delete --all: starting"
    );

    // If root does not exist there is nothing to do.
    if !root.exists() {
        debug!(root = %root_str, "delete --all: root does not exist; nothing to delete");
        Response::new(
            "delete",
            DeleteData {
                root: root_str.to_string(),
                dry_run,
                deleted: 0,
                skipped: 0,
                jobs: vec![],
            },
        )
        .print();
        return Ok(());
    }

    let read_dir = std::fs::read_dir(root)
        .map_err(|e| anyhow!("failed to read root directory {}: {}", root_str, e))?;

    let mut job_results: Vec<DeleteJobResult> = Vec::new();
    let mut deleted_count: u64 = 0;
    let mut skipped_count: u64 = 0;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                debug!(error = %e, "delete --all: failed to read directory entry; skipping");
                skipped_count += 1;
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let job_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => {
                debug!(path = %path.display(), "delete --all: cannot get dir name; skipping");
                skipped_count += 1;
                continue;
            }
        };

        // Read meta.json to check cwd.
        let meta_path = path.join("meta.json");
        let meta: Option<crate::schema::JobMeta> = std::fs::read(&meta_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());

        // Filter by cwd: only jobs whose persisted cwd matches the caller's cwd.
        match meta.as_ref().and_then(|m| m.cwd.as_deref()) {
            Some(job_cwd) if job_cwd == current_cwd => {
                // cwd matches; proceed to state check
            }
            _ => {
                debug!(
                    job_id = %job_id,
                    job_cwd = ?meta.as_ref().and_then(|m| m.cwd.as_deref()),
                    current_cwd = %current_cwd,
                    "delete --all: skipping job (cwd mismatch or absent)"
                );
                // Not counted in skipped — just out of scope.
                continue;
            }
        }

        // Read state.json to determine eligibility.
        let state_path = path.join("state.json");
        let state_opt: Option<crate::schema::JobState> = std::fs::read(&state_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());

        let (state_str, status) = match &state_opt {
            Some(s) => (s.status().as_str().to_string(), Some(s.status().clone())),
            None => {
                debug!(job_id = %job_id, "delete --all: state.json missing or unreadable; skipping");
                skipped_count += 1;
                job_results.push(DeleteJobResult {
                    job_id,
                    state: "unknown".to_string(),
                    action: "skipped".to_string(),
                    reason: "state_unreadable".to_string(),
                });
                continue;
            }
        };

        // Only terminal states are eligible for bulk deletion; skip created and running.
        let is_terminal = matches!(
            status.as_ref(),
            Some(JobStatus::Exited) | Some(JobStatus::Killed) | Some(JobStatus::Failed)
        );

        if !is_terminal {
            let reason = match status.as_ref() {
                Some(JobStatus::Running) => "running",
                Some(JobStatus::Created) => "created",
                _ => "non_terminal",
            };
            debug!(job_id = %job_id, state = %state_str, "delete --all: non-terminal job; skipping");
            skipped_count += 1;
            job_results.push(DeleteJobResult {
                job_id,
                state: state_str,
                action: "skipped".to_string(),
                reason: reason.to_string(),
            });
            continue;
        }

        // Eligible terminal job: delete or dry-run.
        let action = if dry_run {
            debug!(job_id = %job_id, "delete --all: dry-run would delete");
            "would_delete"
        } else {
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {
                    debug!(job_id = %job_id, "delete --all: deleted");
                    deleted_count += 1;
                    "deleted"
                }
                Err(e) => {
                    debug!(job_id = %job_id, error = %e, "delete --all: failed to delete; skipping");
                    skipped_count += 1;
                    job_results.push(DeleteJobResult {
                        job_id,
                        state: state_str,
                        action: "skipped".to_string(),
                        reason: format!("delete_failed: {e}"),
                    });
                    continue;
                }
            }
        };

        job_results.push(DeleteJobResult {
            job_id,
            state: state_str,
            action: action.to_string(),
            reason: "terminal_in_cwd".to_string(),
        });
    }

    debug!(
        deleted = deleted_count,
        skipped = skipped_count,
        "delete --all: complete"
    );

    Response::new(
        "delete",
        DeleteData {
            root: root_str.to_string(),
            dry_run,
            deleted: deleted_count,
            skipped: skipped_count,
            jobs: job_results,
        },
    )
    .print();

    Ok(())
}
