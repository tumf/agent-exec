//! Implementation of the `wait` sub-command.
//!
//! Polls `state.json` until the job leaves the `running` state or a timeout
//! is reached.

use anyhow::Result;
use tracing::debug;

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{JobStatus, Response, WaitData};

/// Options for the `wait` sub-command.
#[derive(Debug)]
pub struct WaitOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// Poll interval in milliseconds.
    pub poll_ms: u64,
    /// Total timeout in milliseconds; 0 = wait indefinitely.
    pub timeout_ms: u64,
}

impl<'a> Default for WaitOpts<'a> {
    fn default() -> Self {
        WaitOpts {
            job_id: "",
            root: None,
            poll_ms: 200,
            timeout_ms: 0,
        }
    }
}

/// Execute `wait`: poll until done, then emit JSON.
pub fn execute(opts: WaitOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let poll = std::time::Duration::from_millis(opts.poll_ms);
    let deadline = if opts.timeout_ms > 0 {
        Some(std::time::Instant::now() + std::time::Duration::from_millis(opts.timeout_ms))
    } else {
        None
    };

    loop {
        let state = job_dir.read_state()?;
        debug!(job_id = %opts.job_id, state = ?state.state, "wait poll");

        if state.state != JobStatus::Running {
            let response = Response::new(
                "wait",
                WaitData {
                    job_id: opts.job_id.to_string(),
                    state: state.state.as_str().to_string(),
                    exit_code: state.exit_code,
                },
            );
            response.print();
            return Ok(());
        }

        if let Some(dl) = deadline {
            if std::time::Instant::now() >= dl {
                // Timed out — still running.
                let response = Response::new(
                    "wait",
                    WaitData {
                        job_id: opts.job_id.to_string(),
                        state: JobStatus::Running.as_str().to_string(),
                        exit_code: None,
                    },
                );
                response.print();
                return Ok(());
            }
        }

        std::thread::sleep(poll);
    }
}
