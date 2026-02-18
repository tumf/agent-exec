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
    if let Ok(root) = std::env::var("AGENT_EXEC_ROOT")
        && !root.is_empty()
    {
        return PathBuf::from(root);
    }

    // 3. XDG_DATA_HOME
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("agent-exec").join("jobs");
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

/// Metrics returned by [`JobDir::read_tail_metrics`].
///
/// Bundles the tail content together with the byte counts used in the
/// `run` snapshot and `tail` JSON responses, so that both callers share
/// the same calculation logic.
pub struct TailMetrics {
    /// The tail text (lossy UTF-8, last N lines / max_bytes).
    pub tail: String,
    /// Whether the content was truncated by bytes or lines constraints.
    pub truncated: bool,
    /// Total file size in bytes (0 if the file does not exist).
    pub observed_bytes: u64,
    /// Number of bytes included in `tail`.
    pub included_bytes: u64,
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

    /// Create a new job directory and write `meta.json` atomically.
    pub fn create(root: &std::path::Path, job_id: &str, meta: &JobMeta) -> Result<Self> {
        let path = root.join(job_id);
        std::fs::create_dir_all(&path)
            .with_context(|| format!("create job dir {}", path.display()))?;

        let job_dir = JobDir {
            path,
            job_id: job_id.to_string(),
        };

        job_dir.write_meta_atomic(meta)?;

        Ok(job_dir)
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

    /// Write `meta.json` atomically: write to a temp file then rename.
    pub fn write_meta_atomic(&self, meta: &JobMeta) -> Result<()> {
        let target = self.meta_path();
        let contents = serde_json::to_string_pretty(meta)?;
        write_atomic(&self.path, &target, contents.as_bytes())?;
        Ok(())
    }

    /// Write `state.json` atomically: write to a temp file then rename.
    pub fn write_state(&self, state: &JobState) -> Result<()> {
        let target = self.state_path();
        let contents = serde_json::to_string_pretty(state)?;
        write_atomic(&self.path, &target, contents.as_bytes())?;
        Ok(())
    }

    /// Read the last `max_bytes` of a log file, returning lossy UTF-8.
    pub fn tail_log(&self, filename: &str, tail_lines: u64, max_bytes: u64) -> String {
        self.tail_log_with_truncated(filename, tail_lines, max_bytes)
            .0
    }

    /// Read the last `max_bytes` of a log file, returning (content, truncated).
    /// `truncated` is true when the content was cut by bytes or lines constraints.
    pub fn tail_log_with_truncated(
        &self,
        filename: &str,
        tail_lines: u64,
        max_bytes: u64,
    ) -> (String, bool) {
        let path = self.path.join(filename);
        let Ok(data) = std::fs::read(&path) else {
            return (String::new(), false);
        };

        // Truncate to max_bytes from the end.
        let byte_truncated = data.len() as u64 > max_bytes;
        let start = if byte_truncated {
            (data.len() as u64 - max_bytes) as usize
        } else {
            0
        };
        let slice = &data[start..];

        // Lossy UTF-8 decode.
        let text = String::from_utf8_lossy(slice);

        // Keep only the last tail_lines.
        if tail_lines == 0 {
            return (text.into_owned(), byte_truncated);
        }
        let lines: Vec<&str> = text.lines().collect();
        let skip = lines.len().saturating_sub(tail_lines as usize);
        let line_truncated = skip > 0;
        (lines[skip..].join("\n"), byte_truncated || line_truncated)
    }

    /// Read tail content and byte metrics for a single log file.
    ///
    /// Returns a [`TailMetrics`] that bundles the tail text, truncation flag,
    /// observed file size, and included byte count.  Both `run`'s snapshot
    /// generation and `tail`'s JSON generation use this helper so that the
    /// metric calculation is defined in exactly one place.
    ///
    /// `encoding` is always `"utf-8-lossy"` (as required by the contract).
    pub fn read_tail_metrics(
        &self,
        filename: &str,
        tail_lines: u64,
        max_bytes: u64,
    ) -> TailMetrics {
        let (tail, truncated) = self.tail_log_with_truncated(filename, tail_lines, max_bytes);
        let included_bytes = tail.len() as u64;
        let observed_bytes = std::fs::metadata(self.path.join(filename))
            .map(|m| m.len())
            .unwrap_or(0);
        TailMetrics {
            tail,
            truncated,
            observed_bytes,
            included_bytes,
        }
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
    pub fn init_state(&self, pid: u32, started_at: &str) -> Result<JobState> {
        #[cfg(windows)]
        let windows_job_name = Some(format!("AgentExec-{}", self.job_id));
        #[cfg(not(windows))]
        let windows_job_name: Option<String> = None;

        let state = JobState {
            job: crate::schema::JobStateJob {
                id: self.job_id.clone(),
                status: JobStatus::Running,
                started_at: started_at.to_string(),
            },
            result: crate::schema::JobStateResult {
                exit_code: None,
                signal: None,
                duration_ms: None,
            },
            pid: Some(pid),
            finished_at: None,
            updated_at: crate::run::now_rfc3339_pub(),
            windows_job_name,
        };
        self.write_state(&state)?;
        Ok(state)
    }
}

/// Write `contents` to `target` atomically by writing to a temp file in the
/// same directory and then renaming. This prevents readers from observing a
/// partially-written file.
fn write_atomic(dir: &std::path::Path, target: &std::path::Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;

    // Create a named temporary file in the same directory so that rename is
    // always on the same filesystem (required for atomic rename on POSIX).
    let mut tmp = tempfile::Builder::new()
        .prefix(".tmp-")
        .tempfile_in(dir)
        .with_context(|| format!("create temp file in {}", dir.display()))?;

    tmp.write_all(contents)
        .with_context(|| format!("write temp file for {}", target.display()))?;

    // Persist moves the temp file to the target path atomically.
    tmp.persist(target)
        .map_err(|e| e.error)
        .with_context(|| format!("rename temp file to {}", target.display()))?;

    Ok(())
}

// ---------- Unit tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// Global mutex to serialize tests that mutate process-wide environment variables.
    ///
    /// Rust runs tests in parallel by default; any test that calls `set_var` /
    /// `remove_var` must hold this lock for the duration of the test so that
    /// other env-reading tests do not observe a half-mutated environment.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resolve_root_cli_flag_wins() {
        // CLI flag does not depend on environment variables; no lock needed.
        let root = resolve_root(Some("/tmp/my-root"));
        assert_eq!(root, PathBuf::from("/tmp/my-root"));
    }

    #[test]
    fn resolve_root_env_var() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: guarded by ENV_LOCK; no other env-mutating test runs concurrently.
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: guarded by ENV_LOCK; no other env-mutating test runs concurrently.
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: guarded by ENV_LOCK; no other env-mutating test runs concurrently.
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

    // ---------- Job directory structure tests ----------

    fn make_meta(job_id: &str, root: &std::path::Path) -> crate::schema::JobMeta {
        crate::schema::JobMeta {
            job: crate::schema::JobMetaJob {
                id: job_id.to_string(),
            },
            schema_version: "0.1".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            root: root.display().to_string(),
            env_keys: vec!["FOO".to_string()],
            env_vars: vec![],
            mask: vec![],
        }
    }

    /// Verify that job directory creation writes meta.json and the directory exists.
    #[test]
    fn job_dir_create_writes_meta_json() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let meta = make_meta("test-job-01", root);
        let job_dir = JobDir::create(root, "test-job-01", &meta).unwrap();

        // Directory must exist.
        assert!(job_dir.path.is_dir(), "job directory was not created");

        // meta.json must exist and be parseable.
        assert!(job_dir.meta_path().exists(), "meta.json not found");
        let loaded_meta = job_dir.read_meta().unwrap();
        assert_eq!(loaded_meta.job_id(), "test-job-01");
        assert_eq!(loaded_meta.command, vec!["echo", "hello"]);

        // env_keys must contain key names only (not values).
        assert_eq!(loaded_meta.env_keys, vec!["FOO"]);
    }

    /// Verify that meta.json does NOT contain env values (only keys).
    #[test]
    fn meta_json_env_keys_only_no_values() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut meta = make_meta("test-job-02", root);
        // Simulate env_keys containing only key names (as would be extracted from KEY=VALUE pairs).
        meta.env_keys = vec!["SECRET_KEY".to_string(), "API_TOKEN".to_string()];
        let job_dir = JobDir::create(root, "test-job-02", &meta).unwrap();

        // Read raw JSON to verify values are absent.
        let raw = std::fs::read_to_string(job_dir.meta_path()).unwrap();
        assert!(
            !raw.contains("secret_value"),
            "env value must not be stored in meta.json"
        );
        assert!(raw.contains("SECRET_KEY"), "env key must be stored");
        assert!(raw.contains("API_TOKEN"), "env key must be stored");
    }

    /// Verify that state.json contains updated_at after write_state.
    #[test]
    fn state_json_contains_updated_at() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let meta = make_meta("test-job-03", root);
        let job_dir = JobDir::create(root, "test-job-03", &meta).unwrap();

        let state = crate::schema::JobState {
            job: crate::schema::JobStateJob {
                id: "test-job-03".to_string(),
                status: crate::schema::JobStatus::Running,
                started_at: "2024-01-01T00:00:00Z".to_string(),
            },
            result: crate::schema::JobStateResult {
                exit_code: None,
                signal: None,
                duration_ms: None,
            },
            pid: Some(12345),
            finished_at: None,
            updated_at: "2024-01-01T00:00:01Z".to_string(),
            windows_job_name: None,
        };
        job_dir.write_state(&state).unwrap();

        // Read back and verify.
        assert!(job_dir.state_path().exists(), "state.json not found");
        let loaded = job_dir.read_state().unwrap();
        assert_eq!(loaded.updated_at, "2024-01-01T00:00:01Z");
        assert_eq!(loaded.job_id(), "test-job-03");

        // Also verify the raw JSON contains the updated_at field.
        let raw = std::fs::read_to_string(job_dir.state_path()).unwrap();
        assert!(
            raw.contains("updated_at"),
            "updated_at field missing from state.json"
        );
    }

    /// Verify that write_state uses atomic write (temp file + rename).
    /// We verify this indirectly: the file must not be corrupted even if we
    /// call write_state multiple times rapidly.
    #[test]
    fn state_json_atomic_write_no_corruption() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let meta = make_meta("test-job-04", root);
        let job_dir = JobDir::create(root, "test-job-04", &meta).unwrap();

        for i in 0..10 {
            let state = crate::schema::JobState {
                job: crate::schema::JobStateJob {
                    id: "test-job-04".to_string(),
                    status: crate::schema::JobStatus::Running,
                    started_at: "2024-01-01T00:00:00Z".to_string(),
                },
                result: crate::schema::JobStateResult {
                    exit_code: None,
                    signal: None,
                    duration_ms: None,
                },
                pid: Some(100 + i),
                finished_at: None,
                updated_at: format!("2024-01-01T00:00:{:02}Z", i),
                windows_job_name: None,
            };
            job_dir.write_state(&state).unwrap();

            // Each read must produce valid JSON (no corruption).
            let loaded = job_dir.read_state().unwrap();
            assert_eq!(
                loaded.pid,
                Some(100 + i),
                "state corrupted at iteration {i}"
            );
        }
    }

    /// Verify that meta.json atomic write works correctly.
    #[test]
    fn meta_json_atomic_write() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let meta = make_meta("test-job-05", root);
        let job_dir = JobDir::create(root, "test-job-05", &meta).unwrap();

        // Re-write meta atomically.
        let updated_meta = crate::schema::JobMeta {
            job: crate::schema::JobMetaJob {
                id: "test-job-05".to_string(),
            },
            schema_version: "0.1".to_string(),
            command: vec!["ls".to_string()],
            created_at: "2024-06-01T12:00:00Z".to_string(),
            root: root.display().to_string(),
            env_keys: vec!["PATH".to_string()],
            env_vars: vec![],
            mask: vec![],
        };
        job_dir.write_meta_atomic(&updated_meta).unwrap();

        let loaded = job_dir.read_meta().unwrap();
        assert_eq!(loaded.command, vec!["ls"]);
        assert_eq!(loaded.created_at, "2024-06-01T12:00:00Z");
    }

    /// On non-Windows platforms, `init_state` must write `windows_job_name: None`
    /// (the field is omitted from JSON via `skip_serializing_if`).
    /// On Windows, `init_state` must write the deterministic Job Object name
    /// `"AgentExec-{job_id}"` so that `state.json` always contains the identifier
    /// immediately after `run` returns, without waiting for the supervisor update.
    #[test]
    fn init_state_writes_deterministic_job_name_on_windows() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let job_id = "01TESTJOBID0000000000000";
        let meta = make_meta(job_id, root);
        let job_dir = JobDir::create(root, job_id, &meta).unwrap();
        let state = job_dir.init_state(1234, "2024-01-01T00:00:00Z").unwrap();

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
    }
}
