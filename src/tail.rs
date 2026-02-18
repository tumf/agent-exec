//! Implementation of the `tail` sub-command.

use anyhow::Result;

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{Response, TailData};

/// Options for the `tail` sub-command.
#[derive(Debug)]
pub struct TailOpts<'a> {
    pub job_id: &'a str,
    pub root: Option<&'a str>,
    /// Number of lines to show from the end of each log.
    pub tail_lines: u64,
    /// Maximum bytes to read from the end of each log.
    pub max_bytes: u64,
}

impl<'a> Default for TailOpts<'a> {
    fn default() -> Self {
        TailOpts {
            job_id: "",
            root: None,
            tail_lines: 50,
            max_bytes: 65536,
        }
    }
}

/// Execute `tail`: read log tails and emit JSON.
pub fn execute(opts: TailOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let stdout = job_dir.tail_log("stdout.log", opts.tail_lines, opts.max_bytes);
    let stderr = job_dir.tail_log("stderr.log", opts.tail_lines, opts.max_bytes);

    let response = Response::new(
        "tail",
        TailData {
            job_id: opts.job_id.to_string(),
            stdout,
            stderr,
            encoding: "utf-8-lossy".to_string(),
        },
    );
    response.print();
    Ok(())
}
