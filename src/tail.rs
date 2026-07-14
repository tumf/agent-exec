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
    pub compression_mode: crate::compress::CompressionMode,
}

impl<'a> Default for TailOpts<'a> {
    fn default() -> Self {
        TailOpts {
            job_id: "",
            root: None,
            tail_lines: 50,
            max_bytes: 65536,
            compression_mode: crate::compress::CompressionMode::default(),
        }
    }
}

/// Execute `tail`: read log tails and emit JSON.
pub fn execute(opts: TailOpts) -> Result<()> {
    tail_response(opts)?.print();
    Ok(())
}

pub fn tail_response(opts: TailOpts) -> Result<Response<TailData>> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let stdout_log_path = job_dir.stdout_path();
    let stderr_log_path = job_dir.stderr_path();

    // Use the shared helper so that byte metric calculation is in one place.
    let stdout = job_dir.read_tail_metrics("stdout.log", opts.tail_lines, opts.max_bytes);
    let stderr = job_dir.read_tail_metrics("stderr.log", opts.tail_lines, opts.max_bytes);
    let meta = job_dir.read_meta()?;
    let compression = crate::compress::compress(crate::compress::CompressionInput {
        command: &meta.command,
        stdout: &stdout.tail,
        stderr: &stderr.tail,
        stdout_original_bytes: stdout.observed_bytes,
        stderr_original_bytes: stderr.observed_bytes,
        mode: opts.compression_mode,
    });

    let response = Response::new(
        "tail",
        TailData {
            job_id: job_dir.job_id.clone(),
            stdout: stdout.tail,
            stderr: stderr.tail,
            encoding: "utf-8-lossy".to_string(),
            stdout_log_path: stdout_log_path.display().to_string(),
            stderr_log_path: stderr_log_path.display().to_string(),
            stdout_range: stdout.range,
            stderr_range: stderr.range,
            stdout_total_bytes: stdout.observed_bytes,
            stderr_total_bytes: stderr.observed_bytes,
            compression,
        },
    );
    Ok(response)
}
