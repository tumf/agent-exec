//! Job directory management for agent-exec v0.1.
//!
//! Resolution order for the jobs root:
//!   1. `--root` CLI flag
//!   2. `AGENT_EXEC_ROOT` environment variable
//!   3. `$XDG_DATA_HOME/agent-exec/jobs`
//!   4. `~/.local/share/agent-exec/jobs`

use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::PathBuf;

use crate::schema::{JobMeta, JobState, JobStatus};

/// Sentinel error type to distinguish "job not found" from other I/O errors.
/// Used by callers to emit `error.code = "job_not_found"` instead of `internal_error`.
#[derive(Debug)]
pub struct JobNotFound(pub String);

impl std::fmt::Display for JobNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "job not found: {}", self.0)
    }
}

impl std::error::Error for JobNotFound {}

/// Resolve the jobs root directory following the priority chain.
pub fn resolve_root(cli_root: Option<&str>) -> PathBuf {
    // 1. CLI flag
    if let Some(root) = cli_root {
        return PathBuf::from(root);
    }

    // 2. Environment variable
    if let Ok(root) = std::env::var("AGENT_EXEC_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }

    // 3. XDG_DATA_HOME
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("agent-exec").join("jobs");
        }
    }

    // 4. Default: ~/.local/share/agent-exec/jobs
    //    (On Windows use data_local_dir() as base)
    if let Some(base_dirs) = BaseDirs::new() {
        #[cfg(windows)]
        let base = base_dirs.data_local_dir().to_path_buf();
        #[cfg(not(windows))]
        let base = base_dirs.home_dir().join(".local").join("share");
        return base.join("agent-exec").join("jobs");
    }

    // Fallback if directories crate returns None
    PathBuf::from("~/.local/share/agent-exec/jobs")
}

/// Handle to a specific job's directory.
pub struct JobDir {
    pub path: PathBuf,
    pub job_id: String,
}

impl JobDir {
    /// Open an existing job directory by ID.
    ///
    /// Returns `Err` wrapping `JobNotFound` when the directory does not exist,
    /// so callers can emit `error.code = "job_not_found"` rather than `internal_error`.
    pub fn open(root: &std::path::Path, job_id: &str) -> Result<Self> {
        let path = root.join(job_id);
        if !path.exists() {
            return Err(anyhow::Error::new(JobNotFound(job_id.to_string())));
        }
        Ok(JobDir {
            path,
            job_id: job_id.to_string(),
        })
    }

    /// Create a new job directory and write `meta.json`.
    pub fn create(root: &std::path::Path, job_id: &str, meta: &JobMeta) -> Result<Self> {
        let path = root.join(job_id);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("create job dir {}", path.display()))?;

        let meta_path = path.join("meta.json");
        let contents = serde_json::to_string_pretty(meta)?;
        std::fs::write(&meta_path, contents)
            .with_context(|| format!("write meta.json at {}", meta_path.display()))?;

        Ok(JobDir {
            path,
            job_id: job_id.to_string(),
        })
    }

    pub fn meta_path(&self) -> PathBuf {
        self.path.join("meta.json")
    }
    pub fn state_path(&self) -> PathBuf {
        self.path.join("state.json")
    }
    pub fn stdout_path(&self) -> PathBuf {
        self.path.join("stdout.log")
    }
    pub fn stderr_path(&self) -> PathBuf {
        self.path.join("stderr.log")
    }
    pub fn full_log_path(&self) -> PathBuf {
        self.path.join("full.log")
    }

    pub fn read_meta(&self) -> Result<JobMeta> {
        let raw = std::fs::read(self.meta_path())?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn read_state(&self) -> Result<JobState> {
        let raw = std::fs::read(self.state_path())?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn write_state(&self, state: &JobState) -> Result<()> {
        let contents = serde_json::to_string_pretty(state)?;
        std::fs::write(self.state_path(), contents)?;
        Ok(())
    }

    /// Read the last `max_bytes` of a log file, returning lossy UTF-8.
    pub fn tail_log(&self, filename: &str, tail_lines: u64, max_bytes: u64) -> String {
        let path = self.path.join(filename);
        let Ok(data) = std::fs::read(&path) else {
            return String::new();
        };

        // Truncate to max_bytes from the end.
        let start = if data.len() as u64 > max_bytes {
            (data.len() as u64 - max_bytes) as usize
        } else {
            0
        };
        let slice = &data[start..];

        // Lossy UTF-8 decode.
        let text = String::from_utf8_lossy(slice);

        // Keep only the last tail_lines.
        if tail_lines == 0 {
            return text.into_owned();
        }
        let lines: Vec<&str> = text.lines().collect();
        let skip = lines.len().saturating_sub(tail_lines as usize);
        lines[skip..].join("\n")
    }

    /// Write the initial JobState (running, supervisor PID) to disk.
    ///
    /// This is called by the `run` command immediately after the supervisor
    /// process is spawned, so `pid` is the supervisor's PID. The child process
    /// PID and, on Windows, the Job Object name are not yet known at this point.
    ///
    /// On Windows, the Job Object name is derived deterministically from the
    /// job_id as `"AgentExec-{job_id}"`. This name is written immediately to
    /// `state.json` so that callers reading state after `run` returns can
    /// always find the Job Object identifier, without waiting for the supervisor
    /// to perform its first `write_state` call. The supervisor will confirm the
    /// same name (or update to `failed`) after it successfully assigns the child
    /// process to the named Job Object.
    pub fn init_state(&self, pid: u32) -> Result<JobState> {
        #[cfg(windows)]
        let windows_job_name = Some(format!("AgentExec-{}", self.job_id));
        #[cfg(not(windows))]
        let windows_job_name: Option<String> = None;

        let state = JobState {
            state: JobStatus::Running,
            pid: Some(pid),
            exit_code: None,
            finished_at: None,
            windows_job_name,
        };
        self.write_state(&state)?;
        Ok(state)
    }
}

// ---------- Unit tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_root_cli_flag_wins() {
        let root = resolve_root(Some("/tmp/my-root"));
        assert_eq!(root, PathBuf::from("/tmp/my-root"));
    }

    #[test]
    fn resolve_root_env_var() {
        // SAFETY: test-only; not running in parallel with other env-mutating tests.
        unsafe {
            std::env::set_var("AGENT_EXEC_ROOT", "/tmp/env-root");
            // Also clear XDG to avoid interference.
            std::env::remove_var("XDG_DATA_HOME");
        }
        // CLI flag is None, so env var should win.
        let root = resolve_root(None);
        // Restore.
        unsafe {
            std::env::remove_var("AGENT_EXEC_ROOT");
        }
        assert_eq!(root, PathBuf::from("/tmp/env-root"));
    }

    #[test]
    fn resolve_root_xdg() {
        // SAFETY: test-only; not running in parallel with other env-mutating tests.
        unsafe {
            std::env::remove_var("AGENT_EXEC_ROOT");
            std::env::set_var("XDG_DATA_HOME", "/tmp/xdg");
        }
        let root = resolve_root(None);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
        assert_eq!(root, PathBuf::from("/tmp/xdg/agent-exec/jobs"));
    }

    #[test]
    fn resolve_root_default_contains_agent_exec() {
        // SAFETY: test-only; not running in parallel with other env-mutating tests.
        unsafe {
            std::env::remove_var("AGENT_EXEC_ROOT");
            std::env::remove_var("XDG_DATA_HOME");
        }
        let root = resolve_root(None);
        let root_str = root.to_string_lossy();
        assert!(
            root_str.contains("agent-exec"),
            "expected agent-exec in path, got {root_str}"
        );
    }

    /// On non-Windows platforms, `init_state` must write `windows_job_name: None`
    /// (the field is omitted from JSON via `skip_serializing_if`).
    /// On Windows, `init_state` must write the deterministic Job Object name
    /// `"AgentExec-{job_id}"` so that `state.json` always contains the identifier
    /// immediately after `run` returns, without waiting for the supervisor update.
    #[test]
    fn init_state_writes_deterministic_job_name_on_windows() {
        let tmp = std::env::temp_dir().join(format!(
            "agent-exec-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let job_id = "01TESTJOBID0000000000000";
        let meta = crate::schema::JobMeta {
            job_id: job_id.to_string(),
            schema_version: crate::schema::SCHEMA_VERSION.to_string(),
            command: vec!["echo".to_string()],
            started_at: "2024-01-01T00:00:00Z".to_string(),
            root: tmp.display().to_string(),
        };
        let job_dir = JobDir::create(&tmp, job_id, &meta).unwrap();
        let state = job_dir.init_state(1234).unwrap();

        // Verify in-memory state.
        #[cfg(windows)]
        assert_eq!(
            state.windows_job_name.as_deref(),
            Some("AgentExec-01TESTJOBID0000000000000"),
            "Windows: init_state must set deterministic job name immediately"
        );
        #[cfg(not(windows))]
        assert_eq!(
            state.windows_job_name, None,
            "non-Windows: init_state must not set windows_job_name"
        );

        // Verify persisted state on disk.
        let persisted = job_dir.read_state().unwrap();
        #[cfg(windows)]
        assert_eq!(
            persisted.windows_job_name.as_deref(),
            Some("AgentExec-01TESTJOBID0000000000000"),
            "Windows: persisted state.json must contain windows_job_name"
        );
        #[cfg(not(windows))]
        assert_eq!(
            persisted.windows_job_name, None,
            "non-Windows: persisted state.json must not contain windows_job_name"
        );

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
