//! Implementation of the `gc` sub-command and shared auto-GC engine.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::jobstore::resolve_root;
use crate::schema::{GcData, JobState, JobStatus, Response};

const DEFAULT_OLDER_THAN: &str = "30d";
const DEFAULT_AUTO_SCAN_LIMIT: usize = 200;
const DEFAULT_AUTO_DELETE_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcMode {
    Manual,
    Automatic,
}

#[derive(Debug, Clone)]
pub struct GcPolicy {
    pub older_than: String,
    pub max_jobs: Option<usize>,
    pub max_bytes: Option<u64>,
    pub dry_run: bool,
    pub mode: GcMode,
    pub scan_limit: Option<usize>,
    pub delete_limit: Option<usize>,
}

#[derive(Debug)]
pub struct GcOpts<'a> {
    pub root: Option<&'a str>,
    pub older_than: Option<&'a str>,
    pub max_jobs: Option<u64>,
    pub max_bytes: Option<u64>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
struct Candidate {
    job_id: String,
    path: PathBuf,
    gc_ts: String,
    bytes: u64,
    reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoGcConfig {
    pub enabled: bool,
    pub older_than: String,
    pub max_jobs: Option<usize>,
    pub max_bytes: Option<u64>,
    pub scan_limit: usize,
    pub delete_limit: usize,
}

impl Default for AutoGcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            older_than: DEFAULT_OLDER_THAN.to_string(),
            max_jobs: None,
            max_bytes: None,
            scan_limit: DEFAULT_AUTO_SCAN_LIMIT,
            delete_limit: DEFAULT_AUTO_DELETE_LIMIT,
        }
    }
}

pub fn execute(opts: GcOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let root_str = root.display().to_string();

    let (older_than_str, older_than_source) = match opts.older_than {
        Some(s) => (s.to_string(), "flag".to_string()),
        None => (DEFAULT_OLDER_THAN.to_string(), "default".to_string()),
    };

    let max_jobs = opts
        .max_jobs
        .map(|v| usize::try_from(v).map_err(|_| anyhow!("invalid --max-jobs: {v}")))
        .transpose()?;

    let policy = GcPolicy {
        older_than: older_than_str.clone(),
        max_jobs,
        max_bytes: opts.max_bytes,
        dry_run: opts.dry_run,
        mode: GcMode::Manual,
        scan_limit: None,
        delete_limit: None,
    };

    let outcome = run_gc(&root, &policy)?;

    Response::new(
        "gc",
        GcData {
            root: root_str,
            dry_run: opts.dry_run,
            older_than: older_than_str,
            older_than_source,
            deleted: outcome.deleted,
            skipped: outcome.skipped,
            out_of_scope: outcome.out_of_scope,
            failed: outcome.failed,
            freed_bytes: outcome.freed_bytes,
            scanned_dirs: outcome.scanned_dirs,
            candidate_count: outcome.candidate_count,
        },
    )
    .print();

    Ok(())
}

pub fn maybe_run_auto_gc(root: &Path, cfg: &AutoGcConfig) {
    if !cfg.enabled {
        debug!("auto-gc disabled");
        return;
    }

    let policy = GcPolicy {
        older_than: cfg.older_than.clone(),
        max_jobs: cfg.max_jobs,
        max_bytes: cfg.max_bytes,
        dry_run: false,
        mode: GcMode::Automatic,
        scan_limit: Some(cfg.scan_limit),
        delete_limit: Some(cfg.delete_limit),
    };

    if let Err(e) = run_gc_with_lock(root, &policy) {
        warn!(error = %e, "auto-gc failed (best-effort)");
    }
}

#[derive(Debug)]
struct GcOutcome {
    deleted: u64,
    skipped: u64,
    out_of_scope: u64,
    failed: u64,
    freed_bytes: u64,
    scanned_dirs: u64,
    candidate_count: u64,
}

fn run_gc_with_lock(root: &Path, policy: &GcPolicy) -> Result<GcOutcome> {
    if policy.mode == GcMode::Manual {
        return run_gc(root, policy);
    }

    let lock_path = root.join(".gc.lock");
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path);

    let lock_file = match lock {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            debug!(path = %lock_path.display(), "auto-gc lock already held; skipping");
            return Ok(empty_outcome());
        }
        Err(e) => return Err(anyhow!("create auto-gc lock {}: {e}", lock_path.display())),
    };

    let result = run_gc(root, policy);
    drop(lock_file);
    let _ = std::fs::remove_file(&lock_path);
    result
}

fn empty_outcome() -> GcOutcome {
    GcOutcome {
        deleted: 0,
        skipped: 0,
        out_of_scope: 0,
        failed: 0,
        freed_bytes: 0,
        scanned_dirs: 0,
        candidate_count: 0,
    }
}

fn run_gc(root: &Path, policy: &GcPolicy) -> Result<GcOutcome> {
    if !root.exists() {
        return Ok(empty_outcome());
    }

    let retention_secs = parse_duration(&policy.older_than).ok_or_else(|| {
        anyhow!(
            "invalid duration: {}; expected formats: 30d, 24h, 60m, 3600s",
            policy.older_than
        )
    })?;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff_secs = now_secs.saturating_sub(retention_secs);
    let cutoff = format_rfc3339(cutoff_secs);

    let mut scanned_dirs = 0u64;
    let mut out_of_scope = 0u64;
    let mut skipped = 0u64;
    let mut failed = 0u64;

    let mut candidates = Vec::<Candidate>::new();

    let read_dir = std::fs::read_dir(root)
        .map_err(|e| anyhow!("failed to read root directory {}: {}", root.display(), e))?;

    for entry in read_dir {
        let entry = match entry {
            Ok(v) => v,
            Err(e) => {
                skipped += 1;
                failed += 1;
                warn!(error = %e, "gc: failed to read directory entry");
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        scanned_dirs += 1;
        if let Some(limit) = policy.scan_limit
            && scanned_dirs as usize > limit
        {
            break;
        }

        let job_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => {
                skipped += 1;
                out_of_scope += 1;
                continue;
            }
        };

        let state_path = path.join("state.json");
        let state = match std::fs::read(&state_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<JobState>(&b).ok())
        {
            Some(s) => s,
            None => {
                skipped += 1;
                out_of_scope += 1;
                continue;
            }
        };

        let status = state.status().clone();
        if matches!(status, JobStatus::Running | JobStatus::Created) {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }

        if !matches!(
            status,
            JobStatus::Exited | JobStatus::Killed | JobStatus::Failed
        ) {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }

        let gc_ts = state
            .finished_at
            .as_deref()
            .or(Some(state.updated_at.as_str()))
            .unwrap_or_default()
            .to_string();

        if gc_ts.is_empty() {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }

        if !is_older_than(&gc_ts, &cutoff) {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }

        let bytes = dir_size_bytes(&path);
        candidates.push(Candidate {
            job_id,
            path,
            gc_ts,
            bytes,
            reasons: vec!["older_than"],
        });
    }

    candidates.sort_by(|a, b| a.gc_ts.cmp(&b.gc_ts)); // oldest first

    if let Some(max_jobs) = policy.max_jobs
        && candidates.len() > max_jobs
    {
        // keep newest max_jobs, mark older ones for count pressure
        let cut = candidates.len() - max_jobs;
        for c in &mut candidates[..cut] {
            c.reasons.push("max_jobs");
        }
        for c in &mut candidates[cut..] {
            c.reasons.retain(|r| *r != "older_than");
        }
    }

    if let Some(max_bytes) = policy.max_bytes {
        let mut all_terminal_total = candidates.iter().map(|c| c.bytes).sum::<u64>();
        if all_terminal_total > max_bytes {
            for c in &mut candidates {
                if all_terminal_total <= max_bytes {
                    break;
                }
                if !c.reasons.contains(&"max_bytes") {
                    c.reasons.push("max_bytes");
                }
                all_terminal_total = all_terminal_total.saturating_sub(c.bytes);
            }
        }
    }

    let mut selected = Vec::new();
    for c in candidates {
        if c.reasons.is_empty() {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }
        selected.push(c);
    }

    let candidate_count = selected.len() as u64;
    let mut deleted = 0u64;
    let mut freed_bytes = 0u64;
    let mut deletions = 0usize;

    for c in selected {
        if let Some(limit) = policy.delete_limit
            && deletions >= limit
        {
            skipped += 1;
            out_of_scope += 1;
            continue;
        }

        if policy.dry_run {
            freed_bytes = freed_bytes.saturating_add(c.bytes);
            continue;
        }

        match std::fs::remove_dir_all(&c.path) {
            Ok(()) => {
                if c.path.exists() {
                    skipped += 1;
                    failed += 1;
                } else {
                    deletions += 1;
                    deleted += 1;
                    freed_bytes = freed_bytes.saturating_add(c.bytes);
                }
            }
            Err(e) => {
                skipped += 1;
                failed += 1;
                warn!(job_id = %c.job_id, error = %e, "gc: failed to delete job directory");
            }
        }
    }

    info!(
        mode = ?policy.mode,
        deleted,
        skipped,
        out_of_scope,
        failed,
        freed_bytes,
        scanned_dirs,
        candidate_count,
        "gc complete"
    );

    Ok(GcOutcome {
        deleted,
        skipped,
        out_of_scope,
        failed,
        freed_bytes,
        scanned_dirs,
        candidate_count,
    })
}

pub fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('d') {
        n.parse::<u64>().ok().map(|v| v * 86_400)
    } else if let Some(n) = s.strip_suffix('h') {
        n.parse::<u64>().ok().map(|v| v * 3_600)
    } else if let Some(n) = s.strip_suffix('m') {
        n.parse::<u64>().ok().map(|v| v * 60)
    } else if let Some(n) = s.strip_suffix('s') {
        n.parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

fn is_older_than(ts: &str, cutoff: &str) -> bool {
    let ts_prefix = &ts[..ts.len().min(19)];
    let cutoff_prefix = &cutoff[..cutoff.len().min(19)];
    ts_prefix < cutoff_prefix
}

pub fn dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if let Ok(meta) = p.metadata() {
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                total += dir_size_bytes(&p);
            }
        }
    }
    total
}

fn format_rfc3339(secs: u64) -> String {
    let mut s = secs;
    let seconds = s % 60;
    s /= 60;
    let minutes = s % 60;
    s /= 60;
    let hours = s % 24;
    s /= 24;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_days() {
        assert_eq!(parse_duration("30d"), Some(30 * 86_400));
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("abc").is_none());
    }

    #[test]
    fn older_than_logic() {
        assert!(is_older_than(
            "2020-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z"
        ));
        assert!(!is_older_than(
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z"
        ));
    }
}
