//! Job directory management for agent-exec v0.1.
//!
//! Resolution order for the jobs root:
//!   1. `--root` CLI flag
//!   2. `AGENT_EXEC_ROOT` environment variable
//!   3. `$XDG_DATA_HOME/agent-exec/jobs`
//!   4. `~/.local/share/agent-exec/jobs`

use anyhow::{Context, Result};
use directories::BaseDirs;
use rand::RngCore;
use std::path::{Path, PathBuf};

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

/// Sentinel error type when a job ID prefix matches multiple job directories.
/// Used by callers to emit `error.code = "ambiguous_job_id"` instead of `internal_error`.
#[derive(Debug)]
pub struct AmbiguousJobId {
    pub prefix: String,
    pub candidates: Vec<String>,
}

impl std::fmt::Display for AmbiguousJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ambiguous job ID prefix '{}': matches ", self.prefix)?;
        if self.candidates.len() <= 5 {
            write!(f, "{}", self.candidates.join(", "))
        } else {
            write!(
                f,
                "{}, ... and {} more",
                self.candidates[..5].join(", "),
                self.candidates.len() - 5
            )
        }
    }
}

impl std::error::Error for AmbiguousJobId {}

/// Sentinel error type when job ID generation exhausts all retry attempts.
/// Used by callers to emit `error.code = "io_error"` instead of `internal_error`.
#[derive(Debug)]
pub struct JobIdCollisionExhausted {
    pub attempts: usize,
}

impl std::fmt::Display for JobIdCollisionExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "job ID generation failed: {} consecutive collisions",
            self.attempts
        )
    }
}

impl std::error::Error for JobIdCollisionExhausted {}

/// Sentinel error type for invalid job state transitions.
/// Used by callers to emit `error.code = "invalid_state"` instead of `internal_error`.
#[derive(Debug)]
pub struct InvalidJobState(pub String);

impl std::fmt::Display for InvalidJobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid job state: {}", self.0)
    }
}

impl std::error::Error for InvalidJobState {}

/// Resolve the jobs root directory following the priority chain.
const JOB_ID_HEX_BYTES: usize = 16;
const JOB_ID_LENGTH: usize = JOB_ID_HEX_BYTES * 2;
pub const SHORT_JOB_ID_LENGTH: usize = 7;

const MAX_JOB_ID_ATTEMPTS: usize = 16;

/// Generate a new hash-like job ID (`[0-9a-f]`, fixed-length) that is unique under `root`.
///
/// The generator retries on directory-name collisions up to 16 times.
pub fn generate_job_id(root: &Path) -> Result<String> {
    generate_job_id_with_rng(root, &mut rand::thread_rng())
}

fn generate_job_id_with_rng(root: &Path, rng: &mut impl RngCore) -> Result<String> {
    for _ in 0..MAX_JOB_ID_ATTEMPTS {
        let mut bytes = [0u8; JOB_ID_HEX_BYTES];
        rng.fill_bytes(&mut bytes);
        let candidate = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        debug_assert_eq!(candidate.len(), JOB_ID_LENGTH);

        if !root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(anyhow::Error::new(JobIdCollisionExhausted {
        attempts: MAX_JOB_ID_ATTEMPTS,
    }))
}

/// Human-facing short job ID for list-like displays.
pub fn short_job_id(job_id: &str) -> String {
    job_id.chars().take(SHORT_JOB_ID_LENGTH).collect()
}

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
/// Bundles the tail content together with the raw byte ranges used in the
/// `tail` JSON responses, so that callers share the same calculation logic.
pub struct TailMetrics {
    /// The tail text (lossy UTF-8, last N lines / max_bytes).
    pub tail: String,
    /// Total file size in bytes (0 if the file does not exist).
    pub observed_bytes: u64,
    /// Raw byte range [begin, end) represented by the returned text.
    pub range: [u64; 2],
}

/// Metrics for the head slice of a log file.
pub struct HeadMetrics {
    /// The head text (lossy UTF-8, first max_bytes bytes).
    pub head: String,
    /// Total file size in bytes (0 if the file does not exist).
    pub observed_bytes: u64,
    /// Number of bytes included in `head`.
    pub included_bytes: u64,
    /// Raw byte range [begin, end) represented by the returned text.
    pub range: [u64; 2],
}

/// Handle to a specific job's directory.
#[derive(Debug)]
pub struct JobDir {
    pub path: PathBuf,
    pub job_id: String,
}

impl JobDir {
    /// Open an existing job directory by ID or unambiguous prefix.
    ///
    /// Resolution order:
    /// 1. Exact match: if `root/<job_id>` exists, return it immediately (no scan).
    /// 2. Prefix scan: scan `root/` for directories whose name starts with `job_id`.
    ///    - 0 matches → `Err(JobNotFound)`
    ///    - 1 match   → resolve to that job
    ///    - 2+ matches → `Err(AmbiguousJobId)`
    pub fn open(root: &std::path::Path, job_id: &str) -> Result<Self> {
        // Exact-match fast path: no directory scan needed.
        let path = root.join(job_id);
        if path.is_dir() {
            return Ok(JobDir {
                path,
                job_id: job_id.to_string(),
            });
        }

        // Prefix scan: collect all directories whose name starts with `job_id`.
        let mut candidates: Vec<String> = std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with(job_id) && entry.path().is_dir() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        match candidates.len() {
            0 => Err(anyhow::Error::new(JobNotFound(job_id.to_string()))),
            1 => {
                let resolved = candidates.remove(0);
                let path = root.join(&resolved);
                Ok(JobDir {
                    path,
                    job_id: resolved,
                })
            }
            _ => {
                candidates.sort();
                Err(anyhow::Error::new(AmbiguousJobId {
                    prefix: job_id.to_string(),
                    candidates,
                }))
            }
        }
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
    pub fn completion_event_path(&self) -> PathBuf {
        self.path.join("completion_event.json")
    }
    pub fn notification_events_path(&self) -> PathBuf {
        self.path.join("notification_events.ndjson")
    }

    /// Write `completion_event.json` atomically.
    pub fn write_completion_event_atomic(
        &self,
        record: &crate::schema::CompletionEventRecord,
    ) -> Result<()> {
        let target = self.completion_event_path();
        let contents = serde_json::to_string_pretty(record)?;
        write_atomic(&self.path, &target, contents.as_bytes())?;
        Ok(())
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

    /// Read tail content and raw byte range metrics for a single log file.
    pub fn read_tail_metrics(
        &self,
        filename: &str,
        tail_lines: u64,
        max_bytes: u64,
    ) -> TailMetrics {
        let path = self.path.join(filename);
        let Ok(data) = std::fs::read(&path) else {
            return TailMetrics {
                tail: String::new(),
                observed_bytes: 0,
                range: [0, 0],
            };
        };

        let observed_bytes = data.len() as u64;
        let window_start = observed_bytes.saturating_sub(max_bytes) as usize;
        let window = &data[window_start..];

        let line_start_in_window = if tail_lines == 0 {
            0
        } else {
            let mut chunks: Vec<&[u8]> = window.split(|b| *b == b'\n').collect();
            if window.ends_with(b"\n") {
                let _ = chunks.pop();
            }
            let keep_from = chunks.len().saturating_sub(tail_lines as usize);
            chunks[..keep_from]
                .iter()
                .map(|c| c.len() + 1)
                .sum::<usize>()
        };

        let selected = &window[line_start_in_window..];
        let tail = String::from_utf8_lossy(selected).into_owned();
        let begin = (window_start + line_start_in_window) as u64;

        TailMetrics {
            tail,
            observed_bytes,
            range: [begin, observed_bytes],
        }
    }

    /// Read head content and byte metrics for a single log file.
    ///
    /// Returns the first `max_bytes` bytes (decoded as UTF-8 lossy) with
    /// canonical raw byte range metadata.
    pub fn read_head_metrics(&self, filename: &str, max_bytes: u64) -> HeadMetrics {
        let path = self.path.join(filename);
        let Ok(data) = std::fs::read(&path) else {
            return HeadMetrics {
                head: String::new(),
                observed_bytes: 0,
                included_bytes: 0,
                range: [0, 0],
            };
        };

        let observed_bytes = data.len() as u64;
        let included_len = observed_bytes.min(max_bytes) as usize;
        let head = String::from_utf8_lossy(&data[..included_len]).into_owned();
        let included_bytes = included_len as u64;

        HeadMetrics {
            head,
            observed_bytes,
            included_bytes,
            range: [0, included_bytes],
        }
    }

    /// Write the initial JobState for a `created` (not-yet-started) job.
    ///
    /// The state is `created`, no process has been spawned, and `started_at` is absent.
    pub fn init_state_created(&self) -> Result<JobState> {
        let state = JobState {
            job: crate::schema::JobStateJob {
                id: self.job_id.clone(),
                status: JobStatus::Created,
                started_at: None,
            },
            result: crate::schema::JobStateResult {
                exit_code: None,
                signal: None,
                duration_ms: None,
            },
            pid: None,
            finished_at: None,
            updated_at: crate::run::now_rfc3339_pub(),
            windows_job_name: None,
        };
        self.write_state(&state)?;
        Ok(state)
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
                started_at: Some(started_at.to_string()),
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
            env_vars_runtime: vec![],
            mask: vec![],
            cwd: None,
            notification: None,
            tags: vec![],
            inherit_env: true,
            env_files: vec![],
            timeout_ms: 0,
            kill_after_ms: 0,
            progress_every_ms: 0,
            shell_wrapper: None,
            stdin_file: None,
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
                started_at: Some("2024-01-01T00:00:00Z".to_string()),
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
                    started_at: Some("2024-01-01T00:00:00Z".to_string()),
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
            env_vars_runtime: vec![],
            mask: vec![],
            cwd: None,
            notification: None,
            tags: vec![],
            inherit_env: true,
            env_files: vec![],
            timeout_ms: 0,
            kill_after_ms: 0,
            progress_every_ms: 0,
            shell_wrapper: None,
            stdin_file: None,
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

    // ---------- Prefix-based job ID resolution tests ----------

    #[test]
    fn job_dir_open_exact_match() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let job_id = "01JQXK3M8E5PQRSTVWYZ12ABCD";
        let meta = make_meta(job_id, root);
        JobDir::create(root, job_id, &meta).unwrap();

        let result = JobDir::open(root, job_id).unwrap();
        assert_eq!(result.job_id, job_id);
    }

    #[test]
    fn job_dir_open_unique_prefix_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let job_id = "01JQXK3M8E5PQRSTVWYZ12ABCD";
        let meta = make_meta(job_id, root);
        JobDir::create(root, job_id, &meta).unwrap();

        // Use a unique prefix
        let result = JobDir::open(root, "01JQXK3M").unwrap();
        assert_eq!(result.job_id, job_id);
    }

    #[test]
    fn job_dir_open_not_found_returns_job_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let err = JobDir::open(root, "ZZZZZ").unwrap_err();
        assert!(
            err.downcast_ref::<JobNotFound>().is_some(),
            "expected JobNotFound, got: {err}"
        );
    }

    #[test]
    fn job_dir_open_ambiguous_prefix_returns_ambiguous() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let id_a = "01JQXK3M8EAAA00000000000AA";
        let id_b = "01JQXK3M8EBBB00000000000BB";
        let meta_a = make_meta(id_a, root);
        let meta_b = make_meta(id_b, root);
        JobDir::create(root, id_a, &meta_a).unwrap();
        JobDir::create(root, id_b, &meta_b).unwrap();

        let err = JobDir::open(root, "01JQXK3M8E").unwrap_err();
        let ambiguous = err
            .downcast_ref::<AmbiguousJobId>()
            .expect("expected AmbiguousJobId");
        assert_eq!(ambiguous.prefix, "01JQXK3M8E");
        assert!(ambiguous.candidates.contains(&id_a.to_string()));
        assert!(ambiguous.candidates.contains(&id_b.to_string()));
    }

    #[test]
    fn ambiguous_job_id_display_up_to_5_candidates() {
        let err = AmbiguousJobId {
            prefix: "01J".to_string(),
            candidates: vec![
                "01JAAA".to_string(),
                "01JBBB".to_string(),
                "01JCCC".to_string(),
            ],
        };
        let msg = err.to_string();
        assert!(msg.contains("01J"), "must include prefix: {msg}");
        assert!(msg.contains("01JAAA"), "must list candidates: {msg}");
    }

    #[test]
    fn ambiguous_job_id_display_truncates_beyond_5() {
        let candidates: Vec<String> = (1..=8)
            .map(|i| format!("01JCANDIDATE{i:02}0000000000"))
            .collect();
        let err = AmbiguousJobId {
            prefix: "01J".to_string(),
            candidates,
        };
        let msg = err.to_string();
        assert!(msg.contains("... and 3 more"), "must truncate: {msg}");
    }

    #[test]
    fn generate_job_id_returns_fixed_length_hex() {
        let tmp = tempfile::tempdir().unwrap();
        let id = generate_job_id(tmp.path()).expect("generate job id");
        assert_eq!(id.len(), JOB_ID_LENGTH, "unexpected job id length");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_hexdigit() && c.is_ascii_lowercase() || c.is_ascii_digit()),
            "job id must be lowercase hex: {id}"
        );
    }

    #[test]
    fn generate_job_id_retries_when_collision_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Prime the root with many IDs to make collisions on first try highly likely
        // only if generator does not check existence. This verifies returned ID is unique.
        for _ in 0..64 {
            let id = generate_job_id(root).expect("seed id");
            std::fs::create_dir(root.join(id)).expect("create seeded dir");
        }

        let id = generate_job_id(root).expect("generate non-colliding id");
        assert!(
            !root.join(&id).exists(),
            "generated id must not collide with existing directory"
        );
    }

    /// A deterministic RNG that always produces the same bytes.
    struct FixedRng([u8; JOB_ID_HEX_BYTES]);

    impl rand::RngCore for FixedRng {
        fn next_u32(&mut self) -> u32 {
            unimplemented!()
        }
        fn next_u64(&mut self) -> u64 {
            unimplemented!()
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            dest.copy_from_slice(&self.0[..dest.len()]);
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> std::result::Result<(), rand::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    #[test]
    fn generate_job_id_fails_after_16_collisions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let fixed_bytes = [0xABu8; JOB_ID_HEX_BYTES];
        let colliding_id: String = fixed_bytes.iter().map(|b| format!("{b:02x}")).collect();
        std::fs::create_dir(root.join(&colliding_id)).unwrap();

        let mut rng = FixedRng(fixed_bytes);
        let err = generate_job_id_with_rng(root, &mut rng).unwrap_err();
        let exhausted = err
            .downcast_ref::<JobIdCollisionExhausted>()
            .expect("expected JobIdCollisionExhausted");
        assert_eq!(exhausted.attempts, MAX_JOB_ID_ATTEMPTS);
    }

    #[test]
    fn job_dir_open_unique_prefix_with_mixed_legacy_and_hash_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let legacy_ulid = "01JQXK3M8E5PQRSTVWYZ12ABCD";
        let hash_id = "deadbeefcafebabe1234567890abcdef";

        let meta_legacy = make_meta(legacy_ulid, root);
        let meta_hash = make_meta(hash_id, root);
        JobDir::create(root, legacy_ulid, &meta_legacy).unwrap();
        JobDir::create(root, hash_id, &meta_hash).unwrap();

        let resolved_legacy = JobDir::open(root, "01JQXK3M").unwrap();
        assert_eq!(resolved_legacy.job_id, legacy_ulid);

        let resolved_hash = JobDir::open(root, "deadbee").unwrap();
        assert_eq!(resolved_hash.job_id, hash_id);
    }
}
