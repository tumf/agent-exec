//! Implementation of the `wait` sub-command.
//!
//! Polls `state.json` until the job leaves the `running` state or a timeout
//! is reached.

use anyhow::Result;
use tracing::debug;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{Response, WaitData};

/// Options for the `wait` sub-command.
#[derive(Debug)]
pub struct WaitOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// Poll interval in seconds.
    pub poll_seconds: u64,
    /// Total timeout in seconds (default 30).
    /// Ignored when `forever` is true.
    pub until_seconds: u64,
    /// Wait indefinitely when true.
    pub forever: bool,
}

impl<'a> Default for WaitOpts<'a> {
    fn default() -> Self {
        WaitOpts {
            job_id: "",
            root: None,
            poll_seconds: 1,
            until_seconds: 30,
            forever: false,
        }
    }
}

fn log_file_size(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

pub fn build_wait_data(job_dir: &JobDir, state: &crate::schema::JobState) -> WaitData {
    let stdout_total_bytes = log_file_size(&job_dir.stdout_path());
    let stderr_total_bytes = log_file_size(&job_dir.stderr_path());
    let updated_at = Some(state.updated_at.clone());

    WaitData {
        job_id: job_dir.job_id.clone(),
        state: state.status().as_str().to_string(),
        exit_code: state.exit_code(),
        stdout_total_bytes,
        stderr_total_bytes,
        updated_at,
    }
}

/// Execute `wait`: poll until done, then emit JSON.
pub fn execute(opts: WaitOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let poll = std::time::Duration::from_secs(opts.poll_seconds.max(1));
    let deadline = if opts.forever {
        None
    } else {
        Some(std::time::Instant::now() + std::time::Duration::from_secs(opts.until_seconds))
    };

    loop {
        let state = job_dir.read_state()?;
        debug!(job_id = %opts.job_id, state = ?state.status(), "wait poll");

        if !state.status().is_non_terminal() {
            let response = Response::new("wait", build_wait_data(&job_dir, &state));
            response.print();
            return Ok(());
        }

        if let Some(dl) = deadline
            && std::time::Instant::now() >= dl
        {
            let mut data = build_wait_data(&job_dir, &state);
            data.exit_code = None;
            let response = Response::new("wait", data);
            response.print();
            return Ok(());
        }

        std::thread::sleep(poll);
    }
}
