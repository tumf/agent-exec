//! Dynamic shell completion support for job ID arguments.
//!
//! # API choice
//!
//! Uses `clap_complete`'s `unstable-dynamic` engine with `ArgValueCompleter`.
//! Each job-ID argument is annotated with a context-specific completer function.
//! At runtime, when the shell calls the binary with `COMPLETE=<shell>` set,
//! `CompleteEnv::complete()` intercepts the invocation and returns candidates.
//!
//! ## Root resolution
//!
//! Completers use `resolve_root(None)` as the primary path, which respects
//! the `AGENT_EXEC_ROOT` environment variable.  For `--root` flag awareness,
//! `resolve_root_for_completion` additionally parses `COMP_LINE` / `COMP_WORDS`
//! (bash) and `_CLAP_COMPLETE_ARGS` to extract a `--root` value when present.
//!
//! ## Resilience
//!
//! All completers are best-effort: if the root is unreadable, if `state.json`
//! is missing or malformed, or if any directory entry fails to read, the
//! offending entry is silently skipped and the remaining candidates are returned.

use clap_complete::engine::CompletionCandidate;
use std::path::PathBuf;

// ── internal helpers ───────────────────────────────────────────────────────────

/// Read the `state` field from `<job_dir>/state.json`.
/// Returns `None` on any I/O or parse failure so callers can treat it as
/// "state unknown" rather than an error.
fn read_job_state(job_dir: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(job_dir.join("state.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value.get("state")?.as_str().map(str::to_string)
}

/// List completion candidates under `root`, optionally filtered by job state.
///
/// - If `root` does not exist or is unreadable, returns an empty list.
/// - If `state_filter` is `Some(slice)`, only jobs whose state appears in
///   the slice are included.  Jobs whose `state.json` is unreadable are
///   excluded when a filter is active (safe default: don't offer jobs we
///   can't categorise).
/// - Each candidate includes the state as a help annotation when available.
pub fn list_job_candidates(
    root: &std::path::Path,
    state_filter: Option<&[&str]>,
) -> Vec<CompletionCandidate> {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let state = read_job_state(&e.path());

            // Apply state filter when specified.
            if let Some(filter) = state_filter {
                match &state {
                    Some(s) if filter.contains(&s.as_str()) => {}
                    _ => return None,
                }
            }

            let candidate = CompletionCandidate::new(name);
            Some(match state {
                Some(s) => candidate.help(Some(s.into())),
                None => candidate,
            })
        })
        .collect()
}

/// Resolve the root directory to use during a completion invocation.
///
/// Tries, in order:
/// 1. `--root <value>` extracted from `COMP_LINE` (bash/zsh).
/// 2. `--root <value>` extracted from the process argv after the `--`
///    separator (covers fish and other shells that pass words as argv).
/// 3. `AGENT_EXEC_ROOT` environment variable (via `resolve_root(None)`).
/// 4. XDG / platform default (via `resolve_root(None)`).
pub fn resolve_root_for_completion() -> PathBuf {
    // Try to extract --root from the partial command line that the shell
    // provides in COMP_LINE (bash/zsh) during completion invocations.
    if let Some(root) = extract_root_from_comp_line() {
        return PathBuf::from(root);
    }
    // Fallback: parse the argv for --root (covers fish and other shells that
    // don't set COMP_LINE but pass completion words as process argv after `--`).
    if let Some(root) = extract_root_from_argv() {
        return PathBuf::from(root);
    }
    crate::jobstore::resolve_root(None)
}

/// Parse `--root <value>` from the process argv (words after the `--` separator).
///
/// In CompleteEnv mode, `clap_complete` invokes the binary as:
///   `<binary> <completer_path> -- <program_name> [args…]`
/// This function looks for `--root` in the words that follow `--` so that
/// shells (e.g. fish) that do not set `COMP_LINE` can still trigger root
/// resolution from an explicit `--root` flag.
fn extract_root_from_argv() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let sep_pos = args.iter().position(|a| a == "--")?;
    let words = &args[sep_pos + 1..];
    let pos = words
        .iter()
        .position(|t| t == "--root" || t.starts_with("--root="))?;

    if let Some(val) = words[pos].strip_prefix("--root=") {
        return Some(val.to_string());
    }
    // `--root <value>` form
    words.get(pos + 1).map(|s| s.to_string())
}

/// Parse `--root <value>` from the `COMP_LINE` environment variable.
///
/// `COMP_LINE` is set by bash/zsh to the full command line being completed.
/// Returns `None` if the variable is absent, malformed, or `--root` is not found.
fn extract_root_from_comp_line() -> Option<String> {
    let comp_line = std::env::var("COMP_LINE").ok()?;
    let tokens: Vec<&str> = comp_line.split_whitespace().collect();
    let pos = tokens
        .iter()
        .position(|&t| t == "--root" || t.starts_with("--root="))?;

    if let Some(tok) = tokens.get(pos)
        && let Some(val) = tok.strip_prefix("--root=")
    {
        return Some(val.to_string());
    }
    // `--root <value>` form
    tokens.get(pos + 1).map(|s| s.to_string())
}

// ── public completer functions ─────────────────────────────────────────────────
//
// Each function matches the signature required by `ArgValueCompleter::new()`:
//   fn(&OsStr) -> Vec<CompletionCandidate>
//
// The `current` parameter is the partial value the user has typed so far.
// clap_complete performs prefix filtering itself, so returning all candidates
// unconditionally is correct.

/// Complete all job IDs regardless of state.
/// Used by: `status`, `tail`, `tag set`, `notify set`.
pub fn complete_all_jobs(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    list_job_candidates(&resolve_root_for_completion(), None)
}

/// Complete only jobs in `created` state.
/// Used by: `start` (only un-started jobs can be started).
pub fn complete_created_jobs(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    list_job_candidates(&resolve_root_for_completion(), Some(&["created"]))
}

/// Complete only jobs in `running` state.
/// Used by: `kill` (only running jobs can be killed).
pub fn complete_running_jobs(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    list_job_candidates(&resolve_root_for_completion(), Some(&["running"]))
}

/// Complete only jobs in terminal states (`exited`, `killed`, `failed`).
/// Used by: `delete` (only finished jobs can be deleted).
pub fn complete_terminal_jobs(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    list_job_candidates(
        &resolve_root_for_completion(),
        Some(&["exited", "killed", "failed"]),
    )
}

/// Complete jobs in non-terminal states (`created`, `running`).
/// Used by: `wait` (waiting on a terminal job is a no-op).
pub fn complete_waitable_jobs(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    list_job_candidates(
        &resolve_root_for_completion(),
        Some(&["created", "running"]),
    )
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_job(root: &std::path::Path, id: &str, state: &str) {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        let state_json = serde_json::json!({ "state": state, "job_id": id });
        fs::write(dir.join("state.json"), state_json.to_string()).unwrap();
    }

    #[test]
    fn test_list_all_jobs_returns_all_dirs() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01AAA", "running");
        make_job(tmp.path(), "01BBB", "exited");

        let candidates = list_job_candidates(tmp.path(), None);
        let names: Vec<_> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"01AAA".to_string()));
        assert!(names.contains(&"01BBB".to_string()));
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_list_with_state_filter() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01AAA", "running");
        make_job(tmp.path(), "01BBB", "exited");
        make_job(tmp.path(), "01CCC", "running");

        let candidates = list_job_candidates(tmp.path(), Some(&["running"]));
        let names: Vec<_> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"01AAA".to_string()));
        assert!(names.contains(&"01CCC".to_string()));
        assert!(!names.contains(&"01BBB".to_string()));
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_nonexistent_root_returns_empty() {
        let candidates = list_job_candidates(std::path::Path::new("/nonexistent/path"), None);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_description_includes_state() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01AAA", "running");

        let candidates = list_job_candidates(tmp.path(), None);
        assert_eq!(candidates.len(), 1);
        let help = candidates[0].get_help();
        assert!(help.is_some());
        assert!(help.unwrap().to_string().contains("running"));
    }

    #[test]
    fn test_missing_state_json_included_without_filter() {
        let tmp = tempdir().unwrap();
        // Job dir without state.json
        fs::create_dir_all(tmp.path().join("01NOSTATE")).unwrap();
        make_job(tmp.path(), "01AAA", "running");

        let candidates = list_job_candidates(tmp.path(), None);
        let names: Vec<_> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"01NOSTATE".to_string()));
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_missing_state_json_excluded_with_filter() {
        let tmp = tempdir().unwrap();
        // Job dir without state.json — should be excluded when filtering
        fs::create_dir_all(tmp.path().join("01NOSTATE")).unwrap();
        make_job(tmp.path(), "01AAA", "running");

        let candidates = list_job_candidates(tmp.path(), Some(&["running"]));
        let names: Vec<_> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect();
        assert!(!names.contains(&"01NOSTATE".to_string()));
        assert!(names.contains(&"01AAA".to_string()));
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn test_terminal_jobs_filter() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01EXITED", "exited");
        make_job(tmp.path(), "01KILLED", "killed");
        make_job(tmp.path(), "01FAILED", "failed");
        make_job(tmp.path(), "01RUNNING", "running");

        let candidates = list_job_candidates(tmp.path(), Some(&["exited", "killed", "failed"]));
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn test_waitable_jobs_filter() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01CREATED", "created");
        make_job(tmp.path(), "01RUNNING", "running");
        make_job(tmp.path(), "01EXITED", "exited");

        let candidates = list_job_candidates(tmp.path(), Some(&["created", "running"]));
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_explicit_root_via_env_var() {
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01AAA", "running");

        // Simulate --root by setting AGENT_EXEC_ROOT
        // SAFETY: single-threaded test; no other threads read AGENT_EXEC_ROOT here.
        unsafe {
            std::env::set_var("AGENT_EXEC_ROOT", tmp.path().to_str().unwrap());
        }
        let root = resolve_root_for_completion();
        unsafe {
            std::env::remove_var("AGENT_EXEC_ROOT");
        }

        let candidates = list_job_candidates(&root, None);
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn test_extract_root_from_comp_line() {
        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("COMP_LINE", "agent-exec --root /tmp/myjobs status ");
        }
        let root = extract_root_from_comp_line();
        unsafe {
            std::env::remove_var("COMP_LINE");
        }
        assert_eq!(root, Some("/tmp/myjobs".to_string()));
    }

    #[test]
    fn test_extract_root_from_comp_line_equals_form() {
        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("COMP_LINE", "agent-exec --root=/tmp/myjobs status ");
        }
        let root = extract_root_from_comp_line();
        unsafe {
            std::env::remove_var("COMP_LINE");
        }
        assert_eq!(root, Some("/tmp/myjobs".to_string()));
    }

    #[test]
    fn test_list_job_candidates_with_explicit_root_path() {
        // Passing an explicit root path directly (not via env) should work.
        let tmp = tempdir().unwrap();
        make_job(tmp.path(), "01CUSTOM", "running");

        let other_tmp = tempdir().unwrap();
        make_job(other_tmp.path(), "01OTHER", "running");

        // list_job_candidates with the explicit root must only return jobs
        // from that root, not from any other location.
        let candidates = list_job_candidates(tmp.path(), None);
        let names: Vec<_> = candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"01CUSTOM".to_string()));
        assert!(!names.contains(&"01OTHER".to_string()));
        assert_eq!(candidates.len(), 1);
    }
}
