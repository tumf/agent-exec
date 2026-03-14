//! Implementation of the `gc` sub-command.
//!
//! Traverses all job directories under the resolved root, evaluates each for
//! eligibility (terminal state + GC timestamp older than the retention window),
//! and either deletes or reports them.

use anyhow::{Result, anyhow};
use tracing::debug;

use crate::jobstore::resolve_root;
use crate::schema::{GcData, GcJobResult, JobStatus, Response};

const DEFAULT_OLDER_THAN: &str = "30d";

/// Options for the `gc` sub-command.
#[derive(Debug)]
pub struct GcOpts<'a> {
    pub root: Option<&'a str>,
    /// Retention duration string (e.g. "30d", "24h"); None means use default.
    pub older_than: Option<&'a str>,
    pub dry_run: bool,
}

/// Execute `gc`: traverse root, evaluate jobs, delete or report, emit JSON.
pub fn execute(opts: GcOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let root_str = root.display().to_string();

    let (older_than_str, older_than_source) = match opts.older_than {
        Some(s) => (s.to_string(), "flag"),
        None => (DEFAULT_OLDER_THAN.to_string(), "default"),
    };

    let retention_secs =
        parse_duration(&older_than_str).ok_or_else(|| anyhow!("invalid duration: {older_than_str}; expected formats: 30d, 24h, 60m, 3600s"))?;

    // Compute the cutoff timestamp as seconds since UNIX epoch.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff_secs = now_secs.saturating_sub(retention_secs);
    let cutoff_rfc3339 = format_rfc3339(cutoff_secs);

    debug!(
        root = %root_str,
        older_than = %older_than_str,
        older_than_source,
        dry_run = opts.dry_run,
        cutoff = %cutoff_rfc3339,
        "gc: starting"
    );

    // If root does not exist, return empty response.
    if !root.exists() {
        debug!(root = %root_str, "gc: root does not exist; nothing to collect");
        Response::new(
            "gc",
            GcData {
                root: root_str,
                dry_run: opts.dry_run,
                older_than: older_than_str,
                older_than_source: older_than_source.to_string(),
                deleted: 0,
                skipped: 0,
                freed_bytes: 0,
                jobs: vec![],
            },
        )
        .print();
        return Ok(());
    }

    let read_dir = std::fs::read_dir(&root)
        .map_err(|e| anyhow!("failed to read root directory {}: {}", root_str, e))?;

    let mut job_results: Vec<GcJobResult> = Vec::new();
    let mut deleted_count: u64 = 0;
    let mut skipped_count: u64 = 0;
    let mut freed_bytes: u64 = 0;

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                debug!(error = %e, "gc: failed to read directory entry; skipping");
                skipped_count += 1;
                continue;
            }
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let job_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => {
                debug!(path = %path.display(), "gc: cannot get dir name; skipping");
                skipped_count += 1;
                continue;
            }
        };

        // Read state.json — required for eligibility evaluation.
        let state_path = path.join("state.json");
        let state = match std::fs::read(&state_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<crate::schema::JobState>(&b).ok())
        {
            Some(s) => s,
            None => {
                debug!(path = %path.display(), "gc: state.json missing or unreadable; skipping");
                skipped_count += 1;
                job_results.push(GcJobResult {
                    job_id,
                    state: "unknown".to_string(),
                    action: "skipped".to_string(),
                    reason: "state_unreadable".to_string(),
                    bytes: 0,
                });
                continue;
            }
        };

        let status = state.status();

        // Running jobs are never deleted.
        if *status == JobStatus::Running {
            debug!(job_id = %job_id, "gc: running job; skipping");
            skipped_count += 1;
            job_results.push(GcJobResult {
                job_id,
                state: "running".to_string(),
                action: "skipped".to_string(),
                reason: "running".to_string(),
                bytes: 0,
            });
            continue;
        }

        // Only terminal states are candidates.
        if !matches!(status, JobStatus::Exited | JobStatus::Killed | JobStatus::Failed) {
            debug!(job_id = %job_id, status = ?status, "gc: unknown status; skipping");
            skipped_count += 1;
            job_results.push(GcJobResult {
                job_id,
                state: status.as_str().to_string(),
                action: "skipped".to_string(),
                reason: "non_terminal_status".to_string(),
                bytes: 0,
            });
            continue;
        }

        // Determine the GC timestamp: finished_at preferred, updated_at as fallback.
        let gc_ts = match state.finished_at.as_deref().or(Some(state.updated_at.as_str())) {
            Some(ts) if !ts.is_empty() => ts.to_string(),
            _ => {
                debug!(job_id = %job_id, "gc: no usable timestamp; skipping");
                skipped_count += 1;
                job_results.push(GcJobResult {
                    job_id,
                    state: status.as_str().to_string(),
                    action: "skipped".to_string(),
                    reason: "no_timestamp".to_string(),
                    bytes: 0,
                });
                continue;
            }
        };

        // Compare GC timestamp to cutoff (lexicographic comparison of RFC 3339 UTC strings).
        if !is_older_than(&gc_ts, &cutoff_rfc3339) {
            debug!(job_id = %job_id, gc_ts = %gc_ts, cutoff = %cutoff_rfc3339, "gc: too recent; skipping");
            skipped_count += 1;
            job_results.push(GcJobResult {
                job_id,
                state: status.as_str().to_string(),
                action: "skipped".to_string(),
                reason: "too_recent".to_string(),
                bytes: 0,
            });
            continue;
        }

        // Compute directory size before deletion.
        let dir_bytes = dir_size_bytes(&path);

        if opts.dry_run {
            debug!(job_id = %job_id, bytes = dir_bytes, "gc: dry-run would delete");
            freed_bytes += dir_bytes;
            job_results.push(GcJobResult {
                job_id,
                state: status.as_str().to_string(),
                action: "would_delete".to_string(),
                reason: format!("older_than_{older_than_str}"),
                bytes: dir_bytes,
            });
        } else {
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {
                    debug!(job_id = %job_id, bytes = dir_bytes, "gc: deleted");
                    deleted_count += 1;
                    freed_bytes += dir_bytes;
                    job_results.push(GcJobResult {
                        job_id,
                        state: status.as_str().to_string(),
                        action: "deleted".to_string(),
                        reason: format!("older_than_{older_than_str}"),
                        bytes: dir_bytes,
                    });
                }
                Err(e) => {
                    debug!(job_id = %job_id, error = %e, "gc: failed to delete; skipping");
                    skipped_count += 1;
                    job_results.push(GcJobResult {
                        job_id,
                        state: status.as_str().to_string(),
                        action: "skipped".to_string(),
                        reason: format!("delete_failed: {e}"),
                        bytes: dir_bytes,
                    });
                }
            }
        }
    }

    debug!(
        deleted = deleted_count,
        skipped = skipped_count,
        freed_bytes,
        "gc: complete"
    );

    Response::new(
        "gc",
        GcData {
            root: root_str,
            dry_run: opts.dry_run,
            older_than: older_than_str,
            older_than_source: older_than_source.to_string(),
            deleted: deleted_count,
            skipped: skipped_count,
            freed_bytes,
            jobs: job_results,
        },
    )
    .print();

    Ok(())
}

/// Parse a duration string into seconds.
///
/// Supported formats: `30d`, `24h`, `60m`, `3600s`.
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
        // Plain number treated as seconds.
        s.parse::<u64>().ok()
    }
}

/// Return true when `ts` represents a point in time strictly before `cutoff`.
///
/// Both `ts` and `cutoff` must be RFC 3339 UTC strings produced by
/// `format_rfc3339` (format: `YYYY-MM-DDTHH:MM:SSZ`).  Lexicographic
/// comparison is correct for zero-padded fixed-width UTC ISO 8601 strings.
fn is_older_than(ts: &str, cutoff: &str) -> bool {
    // Normalize: compare the first 19 chars (YYYY-MM-DDTHH:MM:SS) only so
    // that subsecond suffixes and different timezone markers don't break the
    // comparison.  Both values are UTC so ignoring the suffix is safe.
    let ts_prefix = &ts[..ts.len().min(19)];
    let cutoff_prefix = &cutoff[..cutoff.len().min(19)];
    ts_prefix < cutoff_prefix
}

/// Recursively compute the total byte size of a directory.
///
/// Counts only regular file sizes (metadata size is excluded). Returns 0
/// if the directory cannot be read.
pub fn dir_size_bytes(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if let Ok(meta) = entry_path.metadata() {
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                total += dir_size_bytes(&entry_path);
            }
        }
    }
    total
}

/// Manual conversion of Unix timestamp (seconds) to RFC 3339 UTC string.
///
/// Duplicated from `run.rs` to keep `gc` self-contained.
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
        assert_eq!(parse_duration("7d"), Some(7 * 86_400));
        assert_eq!(parse_duration("1d"), Some(86_400));
    }

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_duration("24h"), Some(24 * 3_600));
        assert_eq!(parse_duration("1h"), Some(3_600));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_duration("60m"), Some(3_600));
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_duration("3600s"), Some(3_600));
        assert_eq!(parse_duration("0s"), Some(0));
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("abc").is_none());
        assert!(parse_duration("").is_none());
    }

    #[test]
    fn is_older_than_true() {
        assert!(is_older_than("2020-01-01T00:00:00Z", "2024-01-01T00:00:00Z"));
    }

    #[test]
    fn is_older_than_false_equal() {
        assert!(!is_older_than("2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z"));
    }

    #[test]
    fn is_older_than_false_newer() {
        assert!(!is_older_than("2025-01-01T00:00:00Z", "2024-01-01T00:00:00Z"));
    }

    #[test]
    fn format_rfc3339_epoch() {
        // UNIX epoch → 1970-01-01T00:00:00Z
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_rfc3339_known() {
        // 2024-01-01T00:00:00Z = 1704067200 secs since epoch
        assert_eq!(format_rfc3339(1_704_067_200), "2024-01-01T00:00:00Z");
    }

    #[test]
    fn dir_size_bytes_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(dir_size_bytes(tmp.path()), 0);
    }

    #[test]
    fn dir_size_bytes_with_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();
        assert_eq!(dir_size_bytes(tmp.path()), 11);
    }
}
