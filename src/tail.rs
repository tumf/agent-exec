//! Implementation of the `tail` sub-command.

use anyhow::Result;

use crate::jobstore::{JobDir, resolve_root};
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

    let stdout_log_path = job_dir.stdout_path();
    let stderr_log_path = job_dir.stderr_path();

    let (stdout_tail, stdout_truncated) =
        job_dir.tail_log_with_truncated("stdout.log", opts.tail_lines, opts.max_bytes);
    let (stderr_tail, stderr_truncated) =
        job_dir.tail_log_with_truncated("stderr.log", opts.tail_lines, opts.max_bytes);
    let truncated = stdout_truncated || stderr_truncated;

    // Byte metrics: observed file sizes and included tail sizes.
    let stdout_observed_bytes = std::fs::metadata(&stdout_log_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let stderr_observed_bytes = std::fs::metadata(&stderr_log_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let stdout_included_bytes = stdout_tail.len() as u64;
    let stderr_included_bytes = stderr_tail.len() as u64;

    let response = Response::new(
        "tail",
        TailData {
            job_id: opts.job_id.to_string(),
            stdout_tail,
            stderr_tail,
            truncated,
            encoding: "utf-8-lossy".to_string(),
            stdout_log_path: stdout_log_path.display().to_string(),
            stderr_log_path: stderr_log_path.display().to_string(),
            stdout_observed_bytes,
            stderr_observed_bytes,
            stdout_included_bytes,
            stderr_included_bytes,
        },
    );
    response.print();
    Ok(())
}
