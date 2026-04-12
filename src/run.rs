//! Implementation of the `run` sub-command.
//!
//! Design decisions (from design.md):
//! - `run` spawns a short-lived front-end that forks a `_supervise` child.
//! - The supervisor continues logging stdout/stderr after `run` returns.
//! - `run` returns launch metadata immediately; observation is delegated to
//!   `status` / `wait` / `tail`.

use anyhow::{Context, Result};
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info, warn};
use ulid::Ulid;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{
    JobMeta, JobMetaJob, JobState, JobStateJob, JobStateResult, JobStatus, Response, RunData,
};
use crate::tag::dedup_tags;

/// Options for the `run` sub-command.
///
/// # Definition-time option alignment rule
///
/// Every definition-time option accepted here MUST also be accepted by `create` (and vice versa),
/// since both commands write the same persisted job definition to `meta.json`. When adding a
/// new persisted metadata field, wire it through both `run` and `create` unless the spec
/// explicitly documents it as launch-only (e.g. snapshot timing, tail sizing, --wait).
#[derive(Debug)]
pub struct RunOpts<'a> {
    /// Command and arguments to execute.
    pub command: Vec<String>,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the command.
    pub cwd: Option<&'a str>,
    /// Environment variables as KEY=VALUE strings.
    pub env_vars: Vec<String>,
    /// Paths to env files, applied in order.
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment (default: true).
    pub inherit_env: bool,
    /// Keys to mask in JSON output (values replaced with "***").
    pub mask: Vec<String>,
    /// Optional stdin source definition persisted in meta and materialized into stdin.bin.
    pub stdin: Option<StdinSource>,
    /// User-defined tags for this job (deduplicated preserving first-seen order).
    pub tags: Vec<String>,
    /// Override full.log path; None = use job dir.
    pub log: Option<&'a str>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// Shell command string for command notification sink; executed via platform shell.
    /// None = no command sink.
    pub notify_command: Option<String>,
    /// File path for NDJSON notification sink; None = no file sink.
    pub notify_file: Option<String>,
    /// Pattern to match against output lines (output-match notification).
    pub output_pattern: Option<String>,
    /// Match type for output-match: "contains" or "regex".
    pub output_match_type: Option<String>,
    /// Stream selector: "stdout", "stderr", or "either".
    pub output_stream: Option<String>,
    /// Shell command string for output-match command sink.
    pub output_command: Option<String>,
    /// File path for output-match NDJSON file sink.
    pub output_file: Option<String>,
    /// Resolved shell wrapper argv used to execute command strings.
    /// e.g. `["sh", "-lc"]` or `["bash", "-lc"]`.
    pub shell_wrapper: Vec<String>,
}

impl<'a> Default for RunOpts<'a> {
    fn default() -> Self {
        RunOpts {
            command: vec![],
            root: None,
            timeout_ms: 0,
            kill_after_ms: 0,
            cwd: None,
            env_vars: vec![],
            env_files: vec![],
            inherit_env: true,
            mask: vec![],
            stdin: None,
            tags: vec![],
            log: None,
            progress_every_ms: 0,
            notify_command: None,
            notify_file: None,
            output_pattern: None,
            output_match_type: None,
            output_stream: None,
            output_command: None,
            output_file: None,
            shell_wrapper: crate::config::default_shell_wrapper(),
        }
    }
}

/// Parameters for spawning a supervisor process.
///
/// Shared by `run::execute` and `start::execute`.
#[derive(Debug, Clone)]
pub enum StdinSource {
    CallerStdin,
    Inline(String),
    File(String),
}

pub struct SpawnSupervisorParams {
    pub job_id: String,
    pub root: std::path::PathBuf,
    pub full_log_path: String,
    pub timeout_ms: u64,
    pub kill_after_ms: u64,
    pub cwd: Option<String>,
    /// Real (unmasked) KEY=VALUE env var pairs.
    pub env_vars: Vec<String>,
    pub env_files: Vec<String>,
    pub inherit_env: bool,
    pub stdin_file: Option<String>,
    pub progress_every_ms: u64,
    pub notify_command: Option<String>,
    pub notify_file: Option<String>,
    pub shell_wrapper: Vec<String>,
    pub command: Vec<String>,
}

pub fn resolve_stdin_source(
    stdin: Option<String>,
    stdin_file: Option<String>,
) -> Option<StdinSource> {
    if let Some(value) = stdin {
        if value == "-" {
            Some(StdinSource::CallerStdin)
        } else {
            Some(StdinSource::Inline(value))
        }
    } else {
        stdin_file.map(StdinSource::File)
    }
}

pub fn validate_stdin_source(stdin: Option<&StdinSource>) -> Result<()> {
    if matches!(stdin, Some(StdinSource::CallerStdin)) {
        let stdin = std::io::stdin();
        if stdin.is_terminal() {
            return Err(anyhow::anyhow!(StdinRequired(
                "stdin_required: --stdin - requires non-interactive stdin (pipe/heredoc/redirect)"
                    .to_string(),
            )));
        }
    }
    Ok(())
}

fn materialize_stdin(job_dir: &JobDir, stdin: Option<&StdinSource>) -> Result<Option<String>> {
    let Some(source) = stdin else {
        return Ok(None);
    };

    let target_name = "stdin.bin".to_string();
    let target_path = job_dir.path.join(&target_name);
    let mut target = std::fs::File::create(&target_path)
        .with_context(|| format!("create materialized stdin {}", target_path.display()))?;

    match source {
        StdinSource::CallerStdin => {
            let mut stdin = std::io::stdin();
            std::io::copy(&mut stdin, &mut target)
                .context("materialize caller stdin to stdin.bin")?;
        }
        StdinSource::Inline(value) => {
            use std::io::Write;
            target
                .write_all(value.as_bytes())
                .context("write inline stdin to stdin.bin")?;
        }
        StdinSource::File(path) => {
            let mut input = std::fs::File::open(path)
                .with_context(|| format!("open --stdin-file source {}", path))?;
            std::io::copy(&mut input, &mut target)
                .with_context(|| format!("copy --stdin-file source {} to stdin.bin", path))?;
        }
    }

    Ok(Some(target_name))
}

fn resolve_stdin_path(job_dir: &JobDir, stdin_file: Option<&str>) -> Option<std::path::PathBuf> {
    stdin_file.map(|p| {
        let path = std::path::Path::new(p);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            job_dir.path.join(path)
        }
    })
}

#[derive(Debug)]
pub struct StdinRequired(pub String);

impl std::fmt::Display for StdinRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StdinRequired {}

pub fn open_child_stdin(job_dir: &JobDir, stdin_file: Option<&str>) -> Result<std::process::Stdio> {
    if let Some(path) = resolve_stdin_path(job_dir, stdin_file) {
        let file = std::fs::File::open(&path)
            .with_context(|| format!("open materialized stdin {}", path.display()))?;
        Ok(std::process::Stdio::from(file))
    } else {
        Ok(std::process::Stdio::null())
    }
}

pub fn materialize_stdin_for_job(
    job_dir: &JobDir,
    stdin: Option<&StdinSource>,
) -> Result<Option<String>> {
    materialize_stdin(job_dir, stdin)
}

/// Spawn the supervisor process and write the initial running state to `state.json`.
///
/// Returns the supervisor PID and the actual `started_at` timestamp.
/// Also handles the Windows Job Object handshake before returning.
pub fn spawn_supervisor_process(
    job_dir: &JobDir,
    params: SpawnSupervisorParams,
) -> Result<(u32, String)> {
    let started_at = now_rfc3339();

    let exe = std::env::current_exe().context("resolve current exe")?;
    let mut supervisor_cmd = Command::new(&exe);
    supervisor_cmd
        .arg("_supervise")
        .arg("--job-id")
        .arg(&params.job_id)
        .arg("--supervise-root")
        .arg(params.root.display().to_string())
        .arg("--full-log")
        .arg(&params.full_log_path);

    if params.timeout_ms > 0 {
        supervisor_cmd
            .arg("--timeout")
            .arg(params.timeout_ms.to_string());
    }
    if params.kill_after_ms > 0 {
        supervisor_cmd
            .arg("--kill-after")
            .arg(params.kill_after_ms.to_string());
    }
    if let Some(ref cwd) = params.cwd {
        supervisor_cmd.arg("--cwd").arg(cwd);
    }
    for env_file in &params.env_files {
        supervisor_cmd.arg("--env-file").arg(env_file);
    }
    for env_var in &params.env_vars {
        supervisor_cmd.arg("--env").arg(env_var);
    }
    if !params.inherit_env {
        supervisor_cmd.arg("--no-inherit-env");
    }
    if let Some(ref stdin_file) = params.stdin_file {
        supervisor_cmd.arg("--stdin-file").arg(stdin_file);
    }
    if params.progress_every_ms > 0 {
        supervisor_cmd
            .arg("--progress-every")
            .arg(params.progress_every_ms.to_string());
    }
    if let Some(ref nc) = params.notify_command {
        supervisor_cmd.arg("--notify-command").arg(nc);
    }
    if let Some(ref nf) = params.notify_file {
        supervisor_cmd.arg("--notify-file").arg(nf);
    }
    let wrapper_json =
        serde_json::to_string(&params.shell_wrapper).context("serialize shell wrapper")?;
    supervisor_cmd
        .arg("--shell-wrapper-resolved")
        .arg(&wrapper_json);

    supervisor_cmd
        .arg("--")
        .args(&params.command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let supervisor = supervisor_cmd.spawn().context("spawn supervisor")?;
    let supervisor_pid = supervisor.id();
    debug!(supervisor_pid, "supervisor spawned");

    // Write initial running state.
    job_dir.init_state(supervisor_pid, &started_at)?;

    // Windows Job Object handshake.
    #[cfg(windows)]
    {
        let handshake_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if let Ok(current_state) = job_dir.read_state() {
                let supervisor_updated = current_state
                    .pid
                    .map(|p| p != supervisor_pid)
                    .unwrap_or(false)
                    || *current_state.status() == crate::schema::JobStatus::Failed;
                if supervisor_updated {
                    if *current_state.status() == crate::schema::JobStatus::Failed {
                        anyhow::bail!(
                            "supervisor failed to assign child process to Job Object \
                             (Windows MUST requirement); see stderr for details"
                        );
                    }
                    debug!("supervisor confirmed Job Object assignment via state.json handshake");
                    break;
                }
            }
            if std::time::Instant::now() >= handshake_deadline {
                debug!("supervisor handshake timed out; proceeding with initial state");
                break;
            }
        }
    }

    Ok((supervisor_pid, started_at))
}

/// Pre-create empty log files (stdout.log, stderr.log, full.log) so they exist
/// immediately after job creation, before the supervisor starts writing.
pub fn pre_create_log_files(job_dir: &JobDir) -> Result<()> {
    for log_path in [
        job_dir.stdout_path(),
        job_dir.stderr_path(),
        job_dir.full_log_path(),
    ] {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("pre-create log file {}", log_path.display()))?;
    }
    Ok(())
}

/// Execute `run`: spawn job and return launch metadata immediately.
pub fn execute(opts: RunOpts) -> Result<()> {
    if opts.command.is_empty() {
        anyhow::bail!("no command specified for run");
    }

    let elapsed_start = std::time::Instant::now();

    let root = resolve_root(opts.root);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("create jobs root {}", root.display()))?;

    let job_id = Ulid::new().to_string();
    let created_at = now_rfc3339();

    // Extract only the key names from KEY=VALUE env var strings (values are not persisted).
    let env_keys: Vec<String> = opts
        .env_vars
        .iter()
        .map(|kv| kv.split('=').next().unwrap_or(kv.as_str()).to_string())
        .collect();

    // Apply masking: replace values of masked keys with "***" in env_vars for metadata.
    let masked_env_vars = mask_env_vars(&opts.env_vars, &opts.mask);

    // Resolve the effective working directory for this job.
    // If --cwd was specified, use that path; otherwise use the current process's working directory.
    // Canonicalize the path for consistent comparison; fall back to absolute path on failure.
    let effective_cwd = resolve_effective_cwd(opts.cwd);

    // Build output-match config from definition-time options (same logic as `create` and `notify set`).
    let on_output_match = crate::notify::build_output_match_config(
        opts.output_pattern,
        opts.output_match_type,
        opts.output_stream,
        opts.output_command,
        opts.output_file,
        None,
    );

    let notification =
        if opts.notify_command.is_some() || opts.notify_file.is_some() || on_output_match.is_some()
        {
            Some(crate::schema::NotificationConfig {
                notify_command: opts.notify_command.clone(),
                notify_file: opts.notify_file.clone(),
                on_output_match,
            })
        } else {
            None
        };

    // Validate and deduplicate tags (preserving first-seen order).
    let tags = dedup_tags(opts.tags)?;

    let meta = JobMeta {
        job: JobMetaJob { id: job_id.clone() },
        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
        command: opts.command.clone(),
        created_at: created_at.clone(),
        root: root.display().to_string(),
        env_keys,
        env_vars: masked_env_vars.clone(),
        // For `run`, env_vars_runtime is not populated because the supervisor
        // is spawned immediately with the real values; no deferred start needed.
        env_vars_runtime: vec![],
        mask: opts.mask.clone(),
        cwd: Some(effective_cwd),
        notification,
        // Execution-definition fields (used by start if ever applicable).
        inherit_env: opts.inherit_env,
        env_files: opts.env_files.clone(),
        timeout_ms: opts.timeout_ms,
        kill_after_ms: opts.kill_after_ms,
        progress_every_ms: opts.progress_every_ms,
        shell_wrapper: Some(opts.shell_wrapper.clone()),
        stdin_file: None,
        tags: tags.clone(),
    };

    validate_stdin_source(opts.stdin.as_ref())?;

    let job_dir = JobDir::create(&root, &job_id, &meta)?;
    let stdin_file = materialize_stdin_for_job(&job_dir, opts.stdin.as_ref())?;
    if stdin_file.is_some() {
        let mut meta_with_stdin = meta.clone();
        meta_with_stdin.stdin_file = stdin_file.clone();
        job_dir.write_meta_atomic(&meta_with_stdin)?;
    }
    info!(job_id = %job_id, "created job directory");

    // Determine the full.log path (may be overridden by --log).
    let full_log_path = if let Some(log) = opts.log {
        log.to_string()
    } else {
        job_dir.full_log_path().display().to_string()
    };

    // Pre-create empty log files so they exist before the supervisor starts.
    pre_create_log_files(&job_dir)?;

    // Spawn the supervisor using the shared helper.
    // Note: masking is handled by `run` (meta.json + JSON response). The supervisor
    // receives the real env var values so the child process can use them as intended.
    let (_supervisor_pid, _started_at) = spawn_supervisor_process(
        &job_dir,
        SpawnSupervisorParams {
            job_id: job_id.clone(),
            root: root.clone(),
            full_log_path: full_log_path.clone(),
            timeout_ms: opts.timeout_ms,
            kill_after_ms: opts.kill_after_ms,
            cwd: opts.cwd.map(|s| s.to_string()),
            env_vars: opts.env_vars.clone(),
            env_files: opts.env_files.clone(),
            inherit_env: opts.inherit_env,
            stdin_file: stdin_file.clone(),
            progress_every_ms: opts.progress_every_ms,
            notify_command: opts.notify_command.clone(),
            notify_file: opts.notify_file.clone(),
            shell_wrapper: opts.shell_wrapper.clone(),
            command: opts.command.clone(),
        },
    )?;

    // Compute absolute paths for stdout.log and stderr.log.
    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    let response = Response::new(
        "run",
        RunData {
            job_id,
            state: JobStatus::Running.as_str().to_string(),
            tags,
            // Include masked env_vars in the JSON response so callers can inspect
            // which variables were set (with secret values replaced by "***").
            env_vars: masked_env_vars,
            stdout_log_path,
            stderr_log_path,
            elapsed_ms,
        },
    );
    response.print();
    Ok(())
}

/// Options for the `_supervise` internal sub-command.
///
/// Masking is the responsibility of `run` (which writes masked values to meta.json
/// and includes them in the JSON response). The supervisor only needs the real
/// environment variable values to launch the child process correctly.
#[derive(Debug)]
pub struct SuperviseOpts<'a> {
    pub job_id: &'a str,
    pub root: &'a Path,
    pub command: &'a [String],
    /// Override full.log path; None = use job dir default.
    pub full_log: Option<&'a str>,
    /// Timeout in milliseconds; 0 = no timeout.
    pub timeout_ms: u64,
    /// Milliseconds after SIGTERM before SIGKILL; 0 = immediate SIGKILL.
    pub kill_after_ms: u64,
    /// Working directory for the child process.
    pub cwd: Option<&'a str>,
    /// Environment variables as KEY=VALUE strings (real values, not masked).
    pub env_vars: Vec<String>,
    /// Paths to env files, applied in order.
    pub env_files: Vec<String>,
    /// Whether to inherit the current process environment.
    pub inherit_env: bool,
    /// Materialized stdin file path (relative to job dir) to feed child stdin.
    pub stdin_file: Option<String>,
    /// Interval (ms) for state.json updated_at refresh; 0 = disabled.
    pub progress_every_ms: u64,
    /// Shell command string for command notification sink; executed via platform shell.
    /// None = no command sink.
    pub notify_command: Option<String>,
    /// File path for NDJSON notification sink; None = no file sink.
    pub notify_file: Option<String>,
    /// Resolved shell wrapper argv used to execute command strings.
    pub shell_wrapper: Vec<String>,
}

/// Resolve the effective working directory for a job.
///
/// If `cwd_override` is `Some`, use that path as the base. Otherwise use the
/// current process working directory. In either case, attempt to canonicalize
/// the path for consistent comparison; on failure, fall back to the absolute
/// path representation (avoids symlink / permission issues on some systems).
pub fn resolve_effective_cwd(cwd_override: Option<&str>) -> String {
    let base = match cwd_override {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };

    // Prefer canonicalized (resolves symlinks); fall back to making the path absolute.
    match base.canonicalize() {
        Ok(canonical) => canonical.display().to_string(),
        Err(_) => {
            // If base is already absolute, use as-is; otherwise prepend cwd.
            if base.is_absolute() {
                base.display().to_string()
            } else {
                // Best-effort: join with cwd, ignore errors.
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                cwd.join(base).display().to_string()
            }
        }
    }
}

/// Mask the values of specified keys in a list of KEY=VALUE strings.
/// Keys listed in `mask_keys` will have their value replaced with "***".
pub fn mask_env_vars(env_vars: &[String], mask_keys: &[String]) -> Vec<String> {
    if mask_keys.is_empty() {
        return env_vars.to_vec();
    }
    env_vars
        .iter()
        .map(|s| {
            let (key, _val) = parse_env_var(s);
            if mask_keys.iter().any(|k| k == &key) {
                format!("{key}=***")
            } else {
                s.clone()
            }
        })
        .collect()
}

/// Parse a single KEY=VALUE or KEY= string into (key, value).
fn parse_env_var(s: &str) -> (String, String) {
    if let Some(pos) = s.find('=') {
        (s[..pos].to_string(), s[pos + 1..].to_string())
    } else {
        (s.to_string(), String::new())
    }
}

/// Load environment variables from a .env-style file.
/// Supports KEY=VALUE lines; lines starting with '#' and empty lines are ignored.
fn load_env_file(path: &str) -> Result<Vec<(String, String)>> {
    let contents =
        std::fs::read_to_string(path).with_context(|| format!("read env-file {path}"))?;
    let mut vars = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        vars.push(parse_env_var(line));
    }
    Ok(vars)
}

/// Shared state for output-match checking, used by streaming threads in `supervise`.
///
/// Reloads `meta.json` on every observed line so that a `notify set` update is
/// visible for the very next line, regardless of how recently the last reload
/// occurred.  Multiple streaming threads share the same checker via `Arc`; the
/// internal `Mutex` serialises access.
struct OutputMatchChecker {
    job_dir_path: std::path::PathBuf,
    shell_wrapper: Vec<String>,
    inner: std::sync::Mutex<OutputMatchInner>,
}

struct OutputMatchInner {
    config: Option<crate::schema::NotificationConfig>,
}

impl OutputMatchChecker {
    fn new(
        job_dir_path: std::path::PathBuf,
        shell_wrapper: Vec<String>,
        initial_config: Option<crate::schema::NotificationConfig>,
    ) -> Self {
        Self {
            job_dir_path,
            shell_wrapper,
            inner: std::sync::Mutex::new(OutputMatchInner {
                config: initial_config,
            }),
        }
    }

    /// Check a newly observed output line for a configured match.
    ///
    /// Reloads `meta.json` on every call so that `notify set` updates are
    /// visible for the next line without any delay.
    /// Dispatches `job.output.matched` events outside the lock to avoid blocking
    /// other streaming threads.
    fn check_line(&self, line: &str, stream: &str) {
        use crate::schema::{OutputMatchStream, OutputMatchType};

        // Lock, reload, evaluate match, then release before dispatching.
        let match_info: Option<crate::schema::OutputMatchConfig> = {
            let mut inner = self.inner.lock().unwrap();

            // Reload config on every line to pick up `notify set` updates immediately.
            {
                let meta_path = self.job_dir_path.join("meta.json");
                if let Ok(raw) = std::fs::read(&meta_path)
                    && let Ok(meta) = serde_json::from_slice::<crate::schema::JobMeta>(&raw)
                {
                    inner.config = meta.notification;
                }
            }

            let Some(ref notification) = inner.config else {
                return;
            };
            let Some(ref match_cfg) = notification.on_output_match else {
                return;
            };

            // Check stream filter.
            let stream_matches = match match_cfg.stream {
                OutputMatchStream::Stdout => stream == "stdout",
                OutputMatchStream::Stderr => stream == "stderr",
                OutputMatchStream::Either => true,
            };
            if !stream_matches {
                return;
            }

            // Check pattern.
            let matched = match &match_cfg.match_type {
                OutputMatchType::Contains => line.contains(&match_cfg.pattern),
                OutputMatchType::Regex => regex::Regex::new(&match_cfg.pattern)
                    .map(|re| re.is_match(line))
                    .unwrap_or(false),
            };

            if matched {
                Some(match_cfg.clone())
            } else {
                None
            }
        }; // Lock released.

        if let Some(match_cfg) = match_info {
            self.dispatch_match(line, stream, &match_cfg);
        }
    }

    /// Dispatch a `job.output.matched` event and append a delivery record to
    /// `notification_events.ndjson`.  Failures are non-fatal.
    fn dispatch_match(
        &self,
        line: &str,
        stream: &str,
        match_cfg: &crate::schema::OutputMatchConfig,
    ) {
        use std::io::Write;

        let job_id = self
            .job_dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let stdout_log_path = self.job_dir_path.join("stdout.log").display().to_string();
        let stderr_log_path = self.job_dir_path.join("stderr.log").display().to_string();
        let events_path = self.job_dir_path.join("notification_events.ndjson");
        let events_path_str = events_path.display().to_string();

        let match_type_str = match &match_cfg.match_type {
            crate::schema::OutputMatchType::Contains => "contains",
            crate::schema::OutputMatchType::Regex => "regex",
        };

        let event = crate::schema::OutputMatchEvent {
            schema_version: crate::schema::SCHEMA_VERSION.to_string(),
            event_type: "job.output.matched".to_string(),
            job_id: job_id.to_string(),
            pattern: match_cfg.pattern.clone(),
            match_type: match_type_str.to_string(),
            stream: stream.to_string(),
            line: line.to_string(),
            stdout_log_path,
            stderr_log_path,
        };

        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let mut delivery_results: Vec<crate::schema::SinkDeliveryResult> = Vec::new();

        if let Some(ref cmd) = match_cfg.command {
            delivery_results.push(dispatch_command_sink(
                cmd,
                &event_json,
                job_id,
                &events_path_str,
                &self.shell_wrapper,
                "job.output.matched",
            ));
        }
        if let Some(ref file_path) = match_cfg.file {
            delivery_results.push(dispatch_file_sink(file_path, &event_json));
        }

        // Append delivery record to notification_events.ndjson.
        let record = crate::schema::OutputMatchEventRecord {
            event,
            delivery_results,
        };
        if let Ok(record_json) = serde_json::to_string(&record)
            && let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&events_path)
        {
            let _ = writeln!(f, "{record_json}");
        }
    }
}

/// Stream bytes from a child process output pipe to an individual log file and
/// to the shared `full.log`.
///
/// Reads byte chunks (not lines) so that output without a trailing newline is
/// still captured in the individual log immediately.  The `full.log` format
/// `"<RFC3339> [LABEL] <line>"` is maintained via a line-accumulation buffer:
/// bytes are appended to the buffer until a newline is found, at which point a
/// formatted line is written to `full.log`.  Any remaining bytes at EOF are
/// flushed as a final line.
///
/// The optional `on_line` callback is invoked for each complete line (without
/// the trailing newline) and is used to drive output-match checking.
///
/// This helper is used by both the stdout and stderr monitoring threads inside
/// [`supervise`], replacing the previously duplicated per-stream implementations.
/// Buffer size (8192 bytes) and newline-split logic are preserved unchanged.
fn stream_to_logs<R, F>(
    stream: R,
    log_path: &std::path::Path,
    full_log: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
    label: &str,
    on_line: Option<F>,
) where
    R: std::io::Read,
    F: Fn(&str),
{
    use std::io::Write;
    let mut log_file = std::fs::File::create(log_path).expect("create stream log file in thread");
    let mut stream = stream;
    let mut buf = [0u8; 8192];
    // Incomplete-line buffer for full.log formatting.
    let mut line_buf: Vec<u8> = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let chunk = &buf[..n];
                // Write raw bytes to the individual log (captures partial lines too).
                let _ = log_file.write_all(chunk);
                // Accumulate bytes for full.log line formatting.
                for &b in chunk {
                    if b == b'\n' {
                        let line = String::from_utf8_lossy(&line_buf);
                        if let Ok(mut fl) = full_log.lock() {
                            let ts = now_rfc3339();
                            let _ = writeln!(fl, "{ts} [{label}] {line}");
                        }
                        if let Some(ref f) = on_line {
                            f(&line);
                        }
                        line_buf.clear();
                    } else {
                        line_buf.push(b);
                    }
                }
            }
            Err(_) => break,
        }
    }
    // Flush any remaining incomplete line to full.log and trigger callback.
    if !line_buf.is_empty() {
        let line = String::from_utf8_lossy(&line_buf);
        if let Ok(mut fl) = full_log.lock() {
            let ts = now_rfc3339();
            let _ = writeln!(fl, "{ts} [{label}] {line}");
        }
        if let Some(ref f) = on_line {
            f(&line);
        }
    }
}

/// Internal supervisor sub-command.
///
/// Runs the target command, streams stdout/stderr to individual log files
/// (`stdout.log`, `stderr.log`) **and** to the combined `full.log`, then
/// updates `state.json` when the process finishes.
///
/// On Windows, the child process is assigned to a named Job Object so that
/// the entire process tree can be terminated with a single `kill` call.
/// The Job Object name is recorded in `state.json` as `windows_job_name`.
pub fn supervise(opts: SuperviseOpts) -> Result<()> {
    use std::sync::{Arc, Mutex};

    let job_id = opts.job_id;
    let root = opts.root;
    let command = opts.command;

    if command.is_empty() {
        anyhow::bail!("supervisor: no command");
    }

    let job_dir = JobDir::open(root, job_id)?;

    // Read meta.json for notification config and cwd (used in completion event).
    let meta = job_dir.read_meta()?;
    // Use the actual execution start time, not meta.created_at.
    // For `run`, these are nearly identical. For `start`, the job may have
    // been created long before it was started.
    let started_at = now_rfc3339();

    // Determine full.log path.
    let full_log_path = if let Some(p) = opts.full_log {
        std::path::PathBuf::from(p)
    } else {
        job_dir.full_log_path()
    };

    // Create the full.log file (shared between stdout/stderr threads).
    // Ensure parent directories exist for custom paths.
    if let Some(parent) = full_log_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir for full.log: {}", parent.display()))?;
    }
    let full_log_file = std::fs::File::create(&full_log_path).context("create full.log")?;
    let full_log = Arc::new(Mutex::new(full_log_file));

    // Execute command through the shell wrapper.
    //
    // Two launch modes:
    //
    //   String mode (command.len() == 1):  The single element is a shell command
    //   string passed as-is to the wrapper (e.g. `"echo hello && ls"` preserves
    //   shell operators).  The wrapper process is the workload boundary.
    //
    //   Argv mode (command.len() > 1):  The wrapper is used for login-shell
    //   environment initialisation but immediately hands off to the target via
    //   `exec "$@"`.  The shell replaces itself so the observed child PID and
    //   lifecycle align with the intended workload, not the wrapper.
    //
    // --notify-command delivery always uses the wrapper in string mode
    // (see dispatch_command_sink); this change only affects job argv launches.
    if opts.shell_wrapper.is_empty() {
        anyhow::bail!("supervisor: shell wrapper must not be empty");
    }
    let mut child_cmd = Command::new(&opts.shell_wrapper[0]);
    if command.len() == 1 {
        // Shell-string mode: pass the command string to the wrapper as-is.
        child_cmd.args(&opts.shell_wrapper[1..]).arg(&command[0]);
    } else {
        // Argv mode: launch the workload via the shell wrapper.
        //
        // On Unix the wrapper hands off to the workload via `exec "$@"` so the
        // shell replaces itself and the observed PID / lifecycle align with the
        // intended workload, not the wrapper.
        //
        // On non-Unix platforms (Windows) there is no POSIX `exec`; the wrapper
        // is invoked in shell-string mode with the argv joined into a single
        // quoted command string, preserving the existing cmd/C semantics.
        #[cfg(unix)]
        {
            // `--` serves as $0; argv elements become $1..$n so `$@` expands
            // to the full workload argv.
            child_cmd
                .args(&opts.shell_wrapper[1..])
                .arg("exec \"$@\"")
                .arg("--")
                .args(command);
        }
        #[cfg(not(unix))]
        {
            // Windows fallback: join argv into a shell-compatible string and
            // pass it to the wrapper as a single command string (same as
            // shell-string mode), so cmd /C semantics are preserved.
            let joined = command
                .iter()
                .map(|a| {
                    if a.contains(' ') {
                        format!("\"{}\"", a)
                    } else {
                        a.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            child_cmd.args(&opts.shell_wrapper[1..]).arg(joined);
        }
    }

    if opts.inherit_env {
        // Start with the current environment (default).
    } else {
        child_cmd.env_clear();
    }

    // Apply env files in order.
    for env_file in &opts.env_files {
        let vars = load_env_file(env_file)?;
        for (k, v) in vars {
            child_cmd.env(&k, &v);
        }
    }

    // Apply --env KEY=VALUE overrides (applied after env-files).
    for env_var in &opts.env_vars {
        let (k, v) = parse_env_var(env_var);
        child_cmd.env(&k, &v);
    }

    // Set working directory if specified.
    if let Some(cwd) = opts.cwd {
        child_cmd.current_dir(cwd);
    }

    // Put the child in its own process group so that timeout signals
    // (SIGTERM / SIGKILL) reach the entire process tree, not just the
    // shell wrapper.  Without this, `sh -lc "sleep 60"` would absorb
    // the signal while the grandchild (`sleep`) keeps running.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setsid is async-signal-safe and called before exec.
        unsafe {
            child_cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    // Spawn the child with piped stdout/stderr so we can tee to logs.
    let child_stdin = open_child_stdin(&job_dir, opts.stdin_file.as_deref())?;
    let mut child = child_cmd
        .stdin(child_stdin)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("supervisor: spawn child")?;

    let pid = child.id();
    info!(job_id, pid, "child process started");

    // On Windows, assign child to a named Job Object for process-tree management.
    // The job name is derived from the job_id so that `kill` can look it up.
    // Assignment is a MUST requirement on Windows: if it fails, the supervisor
    // kills the child process and updates state.json to "failed" before returning
    // an error, so that the run front-end (which may have already returned) can
    // detect the failure via state.json on next poll.
    #[cfg(windows)]
    let windows_job_name = {
        match assign_to_job_object(job_id, pid) {
            Ok(name) => Some(name),
            Err(e) => {
                // Job Object assignment failed. Per design.md this is a MUST
                // requirement on Windows. Kill the child process and update
                // state.json to "failed" so the run front-end can detect it.
                let kill_err = child.kill();
                let _ = child.wait(); // reap to avoid zombies

                let failed_state = JobState {
                    job: JobStateJob {
                        id: job_id.to_string(),
                        status: JobStatus::Failed,
                        started_at: Some(started_at.clone()),
                    },
                    result: JobStateResult {
                        exit_code: None,
                        signal: None,
                        duration_ms: None,
                    },
                    pid: Some(pid),
                    finished_at: Some(now_rfc3339()),
                    updated_at: now_rfc3339(),
                    windows_job_name: None,
                };
                // Best-effort: if writing state fails, we still propagate the
                // original assignment error.
                let _ = job_dir.write_state(&failed_state);

                // Dispatch completion event for the failed state if notifications are configured.
                // This mirrors the dispatch logic in the normal exit path so that callers
                // receive a job.finished event even when the supervisor fails early (Windows only).
                if opts.notify_command.is_some() || opts.notify_file.is_some() {
                    let finished_at_ts =
                        failed_state.finished_at.clone().unwrap_or_else(now_rfc3339);
                    let stdout_log = job_dir.stdout_path().display().to_string();
                    let stderr_log = job_dir.stderr_path().display().to_string();
                    let fail_event = crate::schema::CompletionEvent {
                        schema_version: crate::schema::SCHEMA_VERSION.to_string(),
                        event_type: "job.finished".to_string(),
                        job_id: job_id.to_string(),
                        state: JobStatus::Failed.as_str().to_string(),
                        command: meta.command.clone(),
                        cwd: meta.cwd.clone(),
                        started_at: started_at.clone(),
                        finished_at: finished_at_ts,
                        duration_ms: None,
                        exit_code: None,
                        signal: None,
                        stdout_log_path: stdout_log,
                        stderr_log_path: stderr_log,
                    };
                    let fail_event_json = serde_json::to_string(&fail_event).unwrap_or_default();
                    let fail_event_path = job_dir.completion_event_path().display().to_string();
                    let mut fail_delivery_results: Vec<crate::schema::SinkDeliveryResult> =
                        Vec::new();
                    if let Err(we) = job_dir.write_completion_event_atomic(
                        &crate::schema::CompletionEventRecord {
                            event: fail_event.clone(),
                            delivery_results: vec![],
                        },
                    ) {
                        warn!(
                            job_id,
                            error = %we,
                            "failed to write initial completion_event.json for failed job"
                        );
                    }
                    if let Some(ref shell_cmd) = opts.notify_command {
                        fail_delivery_results.push(dispatch_command_sink(
                            shell_cmd,
                            &fail_event_json,
                            job_id,
                            &fail_event_path,
                            &opts.shell_wrapper,
                            "job.finished",
                        ));
                    }
                    if let Some(ref file_path) = opts.notify_file {
                        fail_delivery_results.push(dispatch_file_sink(file_path, &fail_event_json));
                    }
                    if let Err(we) = job_dir.write_completion_event_atomic(
                        &crate::schema::CompletionEventRecord {
                            event: fail_event,
                            delivery_results: fail_delivery_results,
                        },
                    ) {
                        warn!(
                            job_id,
                            error = %we,
                            "failed to update completion_event.json with delivery results for failed job"
                        );
                    }
                }

                if let Err(ke) = kill_err {
                    return Err(anyhow::anyhow!(
                        "supervisor: failed to assign pid {pid} to Job Object \
                         (Windows MUST requirement): {e}; also failed to kill child: {ke}"
                    ));
                }
                return Err(anyhow::anyhow!(
                    "supervisor: failed to assign pid {pid} to Job Object \
                     (Windows MUST requirement); child process was killed; \
                     consider running outside a nested Job Object environment: {e}"
                ));
            }
        }
    };
    #[cfg(not(windows))]
    let windows_job_name: Option<String> = None;

    // Update state.json with real child PID and Windows Job Object name.
    // On Windows, windows_job_name is always Some at this point (guaranteed
    // by the MUST requirement above), so state.json will always contain the
    // Job Object identifier while the job is running.
    let state = JobState {
        job: JobStateJob {
            id: job_id.to_string(),
            status: JobStatus::Running,
            started_at: Some(started_at.clone()),
        },
        result: JobStateResult {
            exit_code: None,
            signal: None,
            duration_ms: None,
        },
        pid: Some(pid),
        finished_at: None,
        updated_at: now_rfc3339(),
        windows_job_name,
    };
    job_dir.write_state(&state)?;

    let child_start_time = std::time::Instant::now();

    // Take stdout/stderr handles before moving child.
    let child_stdout = child.stdout.take().expect("child stdout piped");
    let child_stderr = child.stderr.take().expect("child stderr piped");

    // Create shared output-match checker from the initial meta notification config.
    let match_checker = std::sync::Arc::new(OutputMatchChecker::new(
        job_dir.path.clone(),
        opts.shell_wrapper.clone(),
        meta.notification.clone(),
    ));

    // Completion channels for log threads: each thread sends `()` after stream_to_logs returns.
    // Used for bounded joins below (allows supervisor to exit promptly when descendants
    // hold inherited pipe ends open indefinitely).
    let (tx_stdout_done, rx_stdout_done) = std::sync::mpsc::channel::<()>();
    let (tx_stderr_done, rx_stderr_done) = std::sync::mpsc::channel::<()>();

    // Thread: read stdout, write to stdout.log and full.log.
    let stdout_log_path = job_dir.stdout_path();
    let full_log_stdout = Arc::clone(&full_log);
    let match_checker_stdout = std::sync::Arc::clone(&match_checker);
    let t_stdout = std::thread::spawn(move || {
        stream_to_logs(
            child_stdout,
            &stdout_log_path,
            full_log_stdout,
            "STDOUT",
            Some(move |line: &str| match_checker_stdout.check_line(line, "stdout")),
        );
        let _ = tx_stdout_done.send(());
    });

    // Thread: read stderr, write to stderr.log and full.log.
    let stderr_log_path = job_dir.stderr_path();
    let full_log_stderr = Arc::clone(&full_log);
    let match_checker_stderr = std::sync::Arc::clone(&match_checker);
    let t_stderr = std::thread::spawn(move || {
        stream_to_logs(
            child_stderr,
            &stderr_log_path,
            full_log_stderr,
            "STDERR",
            Some(move |line: &str| match_checker_stderr.check_line(line, "stderr")),
        );
        let _ = tx_stderr_done.send(());
    });

    // Timeout / kill-after / progress-every handling.
    // We spawn a watcher thread to handle timeout and periodic state.json updates.
    let timeout_ms = opts.timeout_ms;
    let kill_after_ms = opts.kill_after_ms;
    let progress_every_ms = opts.progress_every_ms;
    let state_path = job_dir.state_path();
    let job_id_str = job_id.to_string();

    // Use an atomic flag to signal the watcher thread when the child has exited.
    use std::sync::atomic::{AtomicBool, Ordering};
    let child_done = Arc::new(AtomicBool::new(false));

    let watcher = if timeout_ms > 0 || progress_every_ms > 0 {
        let state_path_clone = state_path.clone();
        let child_done_clone = Arc::clone(&child_done);
        Some(std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let timeout_dur = if timeout_ms > 0 {
                Some(std::time::Duration::from_millis(timeout_ms))
            } else {
                None
            };
            let progress_dur = if progress_every_ms > 0 {
                Some(std::time::Duration::from_millis(progress_every_ms))
            } else {
                None
            };

            let poll_interval = std::time::Duration::from_millis(100);

            loop {
                std::thread::sleep(poll_interval);

                // Exit the watcher loop if the child process has finished.
                if child_done_clone.load(Ordering::Relaxed) {
                    break;
                }

                let elapsed = start.elapsed();

                // Check for timeout.
                if let Some(td) = timeout_dur
                    && elapsed >= td
                {
                    info!(job_id = %job_id_str, "timeout reached, sending SIGTERM to process group");
                    // Send SIGTERM to the entire process group (negative PID).
                    // The child was placed in its own session/group via setsid.
                    #[cfg(unix)]
                    {
                        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGTERM) };
                    }
                    // If kill_after > 0, wait kill_after ms then SIGKILL.
                    if kill_after_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(kill_after_ms));
                        info!(job_id = %job_id_str, "kill-after elapsed, sending SIGKILL to process group");
                        #[cfg(unix)]
                        {
                            unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
                        }
                    } else {
                        // Immediate SIGKILL to the process group.
                        #[cfg(unix)]
                        {
                            unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
                        }
                    }
                    break;
                }

                // Progress-every: update updated_at periodically.
                if let Some(pd) = progress_dur {
                    let elapsed_ms = elapsed.as_millis() as u64;
                    let pd_ms = pd.as_millis() as u64;
                    let poll_ms = poll_interval.as_millis() as u64;
                    if elapsed_ms % pd_ms < poll_ms {
                        // Read, update updated_at, write back.
                        if let Ok(raw) = std::fs::read(&state_path_clone)
                            && let Ok(mut st) =
                                serde_json::from_slice::<crate::schema::JobState>(&raw)
                        {
                            st.updated_at = now_rfc3339();
                            if let Ok(s) = serde_json::to_string_pretty(&st) {
                                let _ = std::fs::write(&state_path_clone, s);
                            }
                        }
                    }
                }
            }
        }))
    } else {
        None
    };

    // Wait for child to finish.
    let exit_status = child.wait().context("wait for child")?;

    // Signal the watcher that the child has finished so it can exit its loop.
    child_done.store(true, Ordering::Relaxed);

    // Persist terminal state immediately after the wrapped root process exits.
    // This must happen BEFORE joining log threads, because log threads block on
    // EOF of stdout/stderr pipes and descendant processes that inherited those
    // pipes may keep them open indefinitely. Persisting state here ensures that
    // `status` and `wait` can observe the terminal state without waiting for
    // all descendants to close their inherited handles.
    let duration_ms = child_start_time.elapsed().as_millis() as u64;
    let exit_code = exit_status.code();
    let finished_at = now_rfc3339();

    // Detect signal-killed processes on Unix for accurate state and completion event.
    #[cfg(unix)]
    let (terminal_status, signal_name) = {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = exit_status.signal() {
            (JobStatus::Killed, Some(sig.to_string()))
        } else {
            (JobStatus::Exited, None)
        }
    };
    #[cfg(not(unix))]
    let (terminal_status, signal_name) = (JobStatus::Exited, None::<String>);

    let state = JobState {
        job: JobStateJob {
            id: job_id.to_string(),
            status: terminal_status.clone(),
            started_at: Some(started_at.clone()),
        },
        result: JobStateResult {
            exit_code,
            signal: signal_name.clone(),
            duration_ms: Some(duration_ms),
        },
        pid: Some(pid),
        finished_at: Some(finished_at.clone()),
        updated_at: now_rfc3339(),
        windows_job_name: None, // not needed after process exits
    };
    job_dir.write_state(&state)?;
    info!(job_id, ?exit_code, "child process finished");

    // Bounded join for log-reader threads.
    //
    // After the wrapped root process exits, we give log threads a short window
    // to drain any remaining output and fire output-match callbacks (which is
    // the common case: no descendants, pipe write-end closes promptly).
    //
    // If a thread does not complete within the window, it is detached (dropped).
    // Detaching means the thread will be killed when the supervisor process exits,
    // which is the correct trade-off: supervisor must not linger indefinitely
    // because a descendant holds an inherited pipe write-end open.
    const LOG_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2000);
    let drain_deadline = std::time::Instant::now() + LOG_DRAIN_TIMEOUT;

    let remaining = drain_deadline
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or(std::time::Duration::ZERO);
    if rx_stdout_done.recv_timeout(remaining).is_ok() {
        let _ = t_stdout.join();
    } else {
        drop(t_stdout); // detach: descendant holds the pipe open
    }

    let remaining = drain_deadline
        .checked_duration_since(std::time::Instant::now())
        .unwrap_or(std::time::Duration::ZERO);
    if rx_stderr_done.recv_timeout(remaining).is_ok() {
        let _ = t_stderr.join();
    } else {
        drop(t_stderr); // detach: descendant holds the pipe open
    }

    // Join watcher if present; it exits promptly once child_done is set.
    if let Some(w) = watcher {
        let _ = w.join();
    }

    // Reload the latest notification config from meta.json to pick up any post-creation
    // updates (e.g. from `notify set` invoked after the job was launched).
    let latest_notification = job_dir.read_meta().ok().and_then(|m| m.notification);
    let (current_notify_command, current_notify_file) = match &latest_notification {
        Some(n) => (n.notify_command.clone(), n.notify_file.clone()),
        None => (None, None),
    };

    // Dispatch completion event to configured notification sinks.
    // Failure here must not alter job state (delivery result is recorded separately).
    let has_notification = current_notify_command.is_some() || current_notify_file.is_some();
    if has_notification {
        let stdout_log = job_dir.stdout_path().display().to_string();
        let stderr_log = job_dir.stderr_path().display().to_string();
        let event = crate::schema::CompletionEvent {
            schema_version: crate::schema::SCHEMA_VERSION.to_string(),
            event_type: "job.finished".to_string(),
            job_id: job_id.to_string(),
            state: terminal_status.as_str().to_string(),
            command: meta.command.clone(),
            cwd: meta.cwd.clone(),
            started_at,
            finished_at,
            duration_ms: Some(duration_ms),
            exit_code,
            signal: signal_name,
            stdout_log_path: stdout_log,
            stderr_log_path: stderr_log,
        };

        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let event_path = job_dir.completion_event_path().display().to_string();
        let mut delivery_results: Vec<crate::schema::SinkDeliveryResult> = Vec::new();

        // Write initial completion_event.json before dispatching sinks.
        if let Err(e) =
            job_dir.write_completion_event_atomic(&crate::schema::CompletionEventRecord {
                event: event.clone(),
                delivery_results: vec![],
            })
        {
            warn!(job_id, error = %e, "failed to write initial completion_event.json");
        }

        if let Some(ref shell_cmd) = current_notify_command {
            delivery_results.push(dispatch_command_sink(
                shell_cmd,
                &event_json,
                job_id,
                &event_path,
                &opts.shell_wrapper,
                "job.finished",
            ));
        }
        if let Some(ref file_path) = current_notify_file {
            delivery_results.push(dispatch_file_sink(file_path, &event_json));
        }

        // Update completion_event.json with delivery results.
        if let Err(e) =
            job_dir.write_completion_event_atomic(&crate::schema::CompletionEventRecord {
                event,
                delivery_results,
            })
        {
            warn!(job_id, error = %e, "failed to update completion_event.json with delivery results");
        }
    }

    Ok(())
}

/// Dispatch the command sink: execute the shell command string via the configured shell wrapper,
/// pass event JSON via stdin, and set AGENT_EXEC_EVENT_PATH / AGENT_EXEC_JOB_ID /
/// AGENT_EXEC_EVENT_TYPE env vars.
///
/// The shell wrapper argv (e.g. `["sh", "-lc"]`) is provided by the caller.
/// The command string is appended as the final argument to the wrapper.
fn dispatch_command_sink(
    shell_cmd: &str,
    event_json: &str,
    job_id: &str,
    event_path: &str,
    shell_wrapper: &[String],
    event_type: &str,
) -> crate::schema::SinkDeliveryResult {
    use std::io::Write;
    let attempted_at = now_rfc3339();
    let target = shell_cmd.to_string();

    if shell_cmd.trim().is_empty() {
        return crate::schema::SinkDeliveryResult {
            sink_type: "command".to_string(),
            target,
            success: false,
            error: Some("empty shell command".to_string()),
            attempted_at,
        };
    }

    if shell_wrapper.is_empty() {
        return crate::schema::SinkDeliveryResult {
            sink_type: "command".to_string(),
            target,
            success: false,
            error: Some("shell wrapper must not be empty".to_string()),
            attempted_at,
        };
    }

    let mut cmd = Command::new(&shell_wrapper[0]);
    cmd.args(&shell_wrapper[1..]).arg(shell_cmd);

    cmd.env("AGENT_EXEC_EVENT_PATH", event_path);
    cmd.env("AGENT_EXEC_JOB_ID", job_id);
    cmd.env("AGENT_EXEC_EVENT_TYPE", event_type);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.spawn() {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(event_json.as_bytes());
            }
            match child.wait() {
                Ok(status) if status.success() => crate::schema::SinkDeliveryResult {
                    sink_type: "command".to_string(),
                    target,
                    success: true,
                    error: None,
                    attempted_at,
                },
                Ok(status) => crate::schema::SinkDeliveryResult {
                    sink_type: "command".to_string(),
                    target,
                    success: false,
                    error: Some(format!("exited with status {status}")),
                    attempted_at,
                },
                Err(e) => crate::schema::SinkDeliveryResult {
                    sink_type: "command".to_string(),
                    target,
                    success: false,
                    error: Some(format!("wait error: {e}")),
                    attempted_at,
                },
            }
        }
        Err(e) => crate::schema::SinkDeliveryResult {
            sink_type: "command".to_string(),
            target,
            success: false,
            error: Some(format!("spawn error: {e}")),
            attempted_at,
        },
    }
}

/// Dispatch the file sink: append event JSON as a single NDJSON line.
/// Creates parent directories automatically.
fn dispatch_file_sink(file_path: &str, event_json: &str) -> crate::schema::SinkDeliveryResult {
    use std::io::Write;
    let attempted_at = now_rfc3339();
    let path = std::path::Path::new(file_path);

    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return crate::schema::SinkDeliveryResult {
            sink_type: "file".to_string(),
            target: file_path.to_string(),
            success: false,
            error: Some(format!("create parent dir: {e}")),
            attempted_at,
        };
    }

    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut f) => match writeln!(f, "{event_json}") {
            Ok(_) => crate::schema::SinkDeliveryResult {
                sink_type: "file".to_string(),
                target: file_path.to_string(),
                success: true,
                error: None,
                attempted_at,
            },
            Err(e) => crate::schema::SinkDeliveryResult {
                sink_type: "file".to_string(),
                target: file_path.to_string(),
                success: false,
                error: Some(format!("write error: {e}")),
                attempted_at,
            },
        },
        Err(e) => crate::schema::SinkDeliveryResult {
            sink_type: "file".to_string(),
            target: file_path.to_string(),
            success: false,
            error: Some(format!("open error: {e}")),
            attempted_at,
        },
    }
}

/// Public alias so other modules can call the timestamp helper.
pub fn now_rfc3339_pub() -> String {
    now_rfc3339()
}

fn now_rfc3339() -> String {
    // Use a simple approach that works without chrono.
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_rfc3339(d.as_secs())
}

fn format_rfc3339(secs: u64) -> String {
    // Manual conversion of Unix timestamp to UTC date-time string.
    let mut s = secs;
    let seconds = s % 60;
    s /= 60;
    let minutes = s % 60;
    s /= 60;
    let hours = s % 24;
    s /= 24;

    // Days since 1970-01-01
    let mut days = s;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0usize;
    for (i, &d) in month_days.iter().enumerate() {
        if days < d {
            month = i;
            break;
        }
        days -= d;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Windows-only: create a named Job Object and assign the given child process
/// to it so that the entire process tree can be terminated via `kill`.
///
/// The Job Object is named `"AgentExec-{job_id}"`. This name is stored in
/// `state.json` so that future `kill` invocations can open the same Job Object
/// by name and call `TerminateJobObject` to stop the whole tree.
///
/// Returns `Ok(name)` on success.  Returns `Err` on failure — the caller
/// (`supervise`) treats failure as a fatal error because reliable process-tree
/// management is a Windows MUST requirement (design.md).
#[cfg(windows)]
fn assign_to_job_object(job_id: &str, pid: u32) -> Result<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};
    use windows::core::HSTRING;

    let job_name = format!("AgentExec-{job_id}");
    let hname = HSTRING::from(job_name.as_str());

    unsafe {
        // Open the child process handle (needed for AssignProcessToJobObject).
        let proc_handle =
            OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, false, pid).map_err(|e| {
                anyhow::anyhow!(
                    "supervisor: OpenProcess(pid={pid}) failed — cannot assign to Job Object: {e}"
                )
            })?;

        // Create a named Job Object.
        let job = match CreateJobObjectW(None, &hname) {
            Ok(h) => h,
            Err(e) => {
                let _ = CloseHandle(proc_handle);
                return Err(anyhow::anyhow!(
                    "supervisor: CreateJobObjectW({job_name}) failed: {e}"
                ));
            }
        };

        // Assign the child process to the Job Object.
        // This can fail if the process is already in another job (e.g. CI/nested).
        // Per design.md, assignment is a MUST on Windows — failure is a fatal error.
        if let Err(e) = AssignProcessToJobObject(job, proc_handle) {
            let _ = CloseHandle(job);
            let _ = CloseHandle(proc_handle);
            return Err(anyhow::anyhow!(
                "supervisor: AssignProcessToJobObject(pid={pid}) failed \
                 (process may already belong to another Job Object, e.g. in a CI environment): {e}"
            ));
        }

        // Keep job handle open for the lifetime of the supervisor so the Job
        // Object remains valid. We intentionally do NOT close it here.
        // The OS will close it automatically when the supervisor exits.
        // (We close proc_handle since we only needed it for assignment.)
        let _ = CloseHandle(proc_handle);
        // Note: job handle is intentionally leaked here to keep the Job Object alive.
        // The handle will be closed when the supervisor process exits.
        std::mem::forget(job);
    }

    info!(job_id, name = %job_name, "supervisor: child assigned to Job Object");
    Ok(job_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_epoch() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_known_date() {
        // 2024-01-01T00:00:00Z = 1704067200
        assert_eq!(format_rfc3339(1704067200), "2024-01-01T00:00:00Z");
    }
}
