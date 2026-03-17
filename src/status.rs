//! Implementation of the `status` sub-command.

use anyhow::Result;
use tracing::debug;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{Response, StatusData};

/// Options for the `status` sub-command.
#[derive(Debug)]
pub struct StatusOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
}

/// Execute `status`: read job state and emit JSON.
pub fn execute(opts: StatusOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let meta = job_dir.read_meta()?;
    let state = job_dir.read_state()?;

    debug!(job_id = %opts.job_id, state = ?state.status(), "status query");

    let response = Response::new(
        "status",
        StatusData {
            job_id: opts.job_id.to_string(),
            state: state.status().as_str().to_string(),
            exit_code: state.exit_code(),
            created_at: meta.created_at,
            started_at: state.started_at().map(|s| s.to_string()),
            finished_at: state.finished_at,
        },
    );
    response.print();
    Ok(())
}
