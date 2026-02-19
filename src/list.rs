//! Implementation of the `list` sub-command.
//!
//! Enumerates job directories under root, reads meta.json and state.json
//! for each, and emits a JSON array sorted by started_at descending.
//! Directories that cannot be parsed as jobs are silently counted in `skipped`.

use anyhow::Result;
use tracing::debug;

use crate::jobstore::resolve_root;
use crate::schema::{JobSummary, ListData, Response};

/// Options for the `list` sub-command.
#[derive(Debug)]
pub struct ListOpts<'a> {
    pub root: Option<&'a str>,
    /// Maximum number of jobs to return; 0 = no limit.
    pub limit: u64,
    /// Optional state filter: running|exited|killed|failed|unknown.
    pub state: Option<&'a str>,
}

/// Execute `list`: enumerate jobs and emit JSON.
pub fn execute(opts: ListOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let root_str = root.display().to_string();

    // If root does not exist, return an empty list (normal termination).
    if !root.exists() {
        debug!(root = %root_str, "root does not exist; returning empty list");
        let response = Response::new(
            "list",
            ListData {
                root: root_str,
                jobs: vec![],
                truncated: false,
                skipped: 0,
            },
        );
        response.print();
        return Ok(());
    }

    // Read directory entries.
    let read_dir = std::fs::read_dir(&root)
        .map_err(|e| anyhow::anyhow!("failed to read root directory {}: {}", root_str, e))?;

    let mut jobs: Vec<JobSummary> = Vec::new();
    let mut skipped: u64 = 0;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                debug!(error = %e, "failed to read directory entry; skipping");
                skipped += 1;
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            // Skip non-directory entries (e.g. stray files in root).
            continue;
        }

        // meta.json must exist and be parseable to consider this a job.
        let meta_path = path.join("meta.json");
        let meta_bytes = match std::fs::read(&meta_path) {
            Ok(b) => b,
            Err(_) => {
                debug!(path = %path.display(), "meta.json missing or unreadable; skipping");
                skipped += 1;
                continue;
            }
        };
        let meta: crate::schema::JobMeta = match serde_json::from_slice(&meta_bytes) {
            Ok(m) => m,
            Err(e) => {
                debug!(path = %path.display(), error = %e, "meta.json parse error; skipping");
                skipped += 1;
                continue;
            }
        };

        // state.json is optional: read if available, continue without it if not.
        let state_opt: Option<crate::schema::JobState> = {
            let state_path = path.join("state.json");
            match std::fs::read(&state_path) {
                Ok(b) => serde_json::from_slice(&b).ok(),
                Err(_) => None,
            }
        };

        let (state_str, exit_code, finished_at, updated_at) = if let Some(ref s) = state_opt {
            (
                s.status().as_str().to_string(),
                s.exit_code(),
                s.finished_at.clone(),
                Some(s.updated_at.clone()),
            )
        } else {
            ("unknown".to_string(), None, None, None)
        };

        jobs.push(JobSummary {
            job_id: meta.job.id.clone(),
            state: state_str,
            exit_code,
            started_at: meta.created_at.clone(),
            finished_at,
            updated_at,
        });
    }

    // Apply state filter before sorting and limiting.
    if let Some(filter_state) = opts.state {
        jobs.retain(|j| j.state == filter_state);
    }

    // Sort by started_at descending; tie-break by job_id descending.
    jobs.sort_by(|a, b| {
        b.started_at
            .cmp(&a.started_at)
            .then_with(|| b.job_id.cmp(&a.job_id))
    });

    // Apply limit.
    let truncated = opts.limit > 0 && jobs.len() as u64 > opts.limit;
    if truncated {
        jobs.truncate(opts.limit as usize);
    }

    debug!(
        root = %root_str,
        count = jobs.len(),
        skipped,
        truncated,
        "list complete"
    );

    let response = Response::new(
        "list",
        ListData {
            root: root_str,
            jobs,
            truncated,
            skipped,
        },
    );
    response.print();
    Ok(())
}
