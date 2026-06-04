use crate::compress::route::DetectedKind;
use crate::compress::util::{
    CompressionCandidate, dedup_lines, diagnostic_blocks, fallback_if_empty, json_shape_summary,
    summarize_text, table_rows,
};

pub fn compress_kind(
    kind: DetectedKind,
    raw_stdout: &str,
    raw_stderr: &str,
) -> CompressionCandidate {
    let (stdout, stderr, mut strategy) = match kind {
        DetectedKind::Errors => (
            extract_error_lines(raw_stdout),
            extract_error_lines(raw_stderr),
            vec!["failure-focus".to_string()],
        ),
        DetectedKind::Tests | DetectedKind::CargoTest | DetectedKind::Pytest => (
            extract_test_lines(raw_stdout),
            extract_test_lines(raw_stderr),
            vec!["test-failure-focus".to_string()],
        ),
        DetectedKind::Logs | DetectedKind::DockerLogs => (
            dedup_lines(raw_stdout),
            dedup_lines(raw_stderr),
            vec!["dedupe-repeated-lines".to_string()],
        ),
        DetectedKind::Git
        | DetectedKind::GitStatus
        | DetectedKind::GitLog
        | DetectedKind::GitDiff
        | DetectedKind::GitShow
        | DetectedKind::GitPush
        | DetectedKind::GitPull
        | DetectedKind::GitBranch
        | DetectedKind::GitStash => (
            summarize_git(kind, raw_stdout),
            summarize_git(kind, raw_stderr),
            vec![format!("{}-summary", kind.as_str())],
        ),
        DetectedKind::Json | DetectedKind::JsonStructure => (
            json_shape_summary(raw_stdout),
            json_shape_summary(raw_stderr),
            vec!["json-structure".to_string()],
        ),
        DetectedKind::Search => (
            summarize_search(raw_stdout),
            summarize_search(raw_stderr),
            vec!["search-summary".to_string()],
        ),
        DetectedKind::Summary => (
            summarize_text(raw_stdout),
            summarize_text(raw_stderr),
            vec!["bounded-summary".to_string()],
        ),
    };

    let stdout = fallback_if_empty(stdout, raw_stdout);
    let stderr = fallback_if_empty(stderr, raw_stderr);
    let omitted = stdout.len() < raw_stdout.len() || stderr.len() < raw_stderr.len();
    if omitted {
        strategy.push("truncation".to_string());
    }

    CompressionCandidate {
        stdout,
        stderr,
        omitted,
        strategy,
    }
}

fn extract_error_lines(text: &str) -> String {
    let keywords = [
        "error",
        "failed",
        "failure",
        "panic",
        "traceback",
        "assert",
        "exception",
    ];
    let lines = filter_lines(text, |line| {
        let lower = line.to_ascii_lowercase();
        keywords.iter().any(|k| lower.contains(k))
    });
    if lines.is_empty() {
        diagnostic_blocks(text)
            .into_iter()
            .take(80)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        lines
    }
}

fn extract_test_lines(text: &str) -> String {
    let mut kept = Vec::new();
    let mut in_failure_section = false;

    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let is_failure_header = lower.contains("failures:")
            || lower.contains("failed")
            || lower.contains("failure")
            || lower.contains("panic")
            || lower.contains("assert");
        if is_failure_header {
            in_failure_section = true;
        }

        let is_summary = lower.contains("test result:")
            || lower.contains(" passed")
            || lower.contains(" failed")
            || lower.contains(" skipped")
            || lower.contains(" ignored");
        let is_passing_test_line = lower.starts_with("test ") && lower.ends_with(" ... ok");
        if in_failure_section || is_summary || (lower.contains("test") && !is_passing_test_line) {
            kept.push(line);
        }

        if kept.len() >= 80 {
            break;
        }
    }

    kept.join("\n")
}

fn filter_lines(text: &str, keep: impl Fn(&str) -> bool) -> String {
    text.lines()
        .filter(|line| keep(line))
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_git(kind: DetectedKind, text: &str) -> String {
    match kind {
        DetectedKind::GitStatus => summarize_git_status(text),
        DetectedKind::GitLog => summarize_git_log(text),
        DetectedKind::GitDiff | DetectedKind::GitShow => summarize_git_diff(text),
        DetectedKind::GitPush => summarize_git_push(text),
        DetectedKind::GitPull => summarize_git_pull(text),
        DetectedKind::GitBranch => summarize_git_branch(text),
        DetectedKind::GitStash => summarize_git_stash(text),
        DetectedKind::Git => summarize_git_generic(text),
        _ => summarize_git_generic(text),
    }
}

fn summarize_git_generic(text: &str) -> String {
    if text.contains("diff --git") || text.contains("@@") {
        summarize_git_diff(text)
    } else if text.contains("commit ") || text.contains(" files changed") {
        summarize_git_log(text)
    } else {
        filter_lines(text, meaningful_git_line)
    }
}

fn meaningful_git_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty()
        && !trimmed.starts_with("hint:")
        && !trimmed.starts_with('(')
        && !trimmed.starts_with("use ")
        && !trimmed.starts_with("nothing added")
}

fn summarize_git_status(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            meaningful_git_line(line)
                && !trimmed.starts_with("Changes to be committed:")
                && !trimmed.starts_with("Changes not staged")
                && !trimmed.starts_with("Untracked files:")
        })
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_git_log(text: &str) -> String {
    let mut out = Vec::new();
    let mut commit_count = 0usize;
    let mut body_lines = 0usize;
    let mut total_files = 0usize;
    let mut total_insertions = 0usize;
    let mut total_deletions = 0usize;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("commit ") {
            commit_count += 1;
            body_lines = 0;
            if commit_count <= 30 {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if is_oneline_commit(trimmed) {
            commit_count += 1;
            body_lines = 0;
            if commit_count <= 30 {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if let Some((files, insertions, deletions)) = parse_shortstat(trimmed) {
            total_files += files;
            total_insertions += insertions;
            total_deletions += deletions;
            if commit_count <= 30 {
                out.push(format!("  {files} files, +{insertions} -{deletions}"));
            }
            continue;
        }
        if commit_count > 0
            && commit_count <= 30
            && body_lines < 3
            && !trimmed.is_empty()
            && !trimmed.starts_with("Author:")
            && !trimmed.starts_with("Date:")
            && !trimmed.starts_with("Signed-off-by:")
            && !trimmed.starts_with("Co-authored-by:")
            && !trimmed.contains(" | ")
        {
            out.push(format!("  {trimmed}"));
            body_lines += 1;
        }
    }
    if commit_count > 30 {
        out.push(format!("... +{} commits omitted", commit_count - 30));
    }
    if total_files > 0 {
        out.push(format!(
            "total: {total_files} files, +{total_insertions} -{total_deletions}"
        ));
    }
    out.join("\n")
}

fn is_oneline_commit(line: &str) -> bool {
    let Some((hash, _subject)) = line.split_once(' ') else {
        return false;
    };
    hash.len() >= 7 && hash.len() <= 40 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_shortstat(line: &str) -> Option<(usize, usize, usize)> {
    if !line.contains("changed") {
        return None;
    }
    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;
    let normalized = line.replace(',', "");
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    for window in parts.windows(2) {
        let Ok(value) = window[0].parse::<usize>() else {
            continue;
        };
        match window[1].trim_end_matches("(+)").trim_end_matches("(-)") {
            "file" | "files" => files = value,
            "insertion" | "insertions" => insertions = value,
            "deletion" | "deletions" => deletions = value,
            _ => {}
        }
    }
    Some((files, insertions, deletions))
}

fn summarize_git_diff(text: &str) -> String {
    let mut out = Vec::new();
    let mut current_file: Option<String> = None;
    let mut plus = 0usize;
    let mut minus = 0usize;
    let mut hunk_body = 0usize;

    for line in text.lines() {
        if line.starts_with("diff --git ") {
            flush_diff_stats(&mut out, current_file.is_some(), &mut plus, &mut minus);
            current_file = line.split_whitespace().nth(3).map(clean_git_path);
            if let Some(file) = &current_file {
                out.push(file.clone());
            }
            continue;
        }
        if line.starts_with("@@") {
            hunk_body = 0;
            out.push(format!("  {line}"));
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("index ") {
            continue;
        }
        if line.starts_with('+') {
            plus += 1;
            if hunk_body < 6 {
                out.push(format!("  {line}"));
                hunk_body += 1;
            }
        } else if line.starts_with('-') {
            minus += 1;
            if hunk_body < 6 {
                out.push(format!("  {line}"));
                hunk_body += 1;
            }
        } else if current_file.is_some() && hunk_body < 3 && !line.trim().is_empty() {
            out.push(format!("  {line}"));
            hunk_body += 1;
        }
    }
    flush_diff_stats(&mut out, current_file.is_some(), &mut plus, &mut minus);
    out.join("\n")
}

fn clean_git_path(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

fn flush_diff_stats(out: &mut Vec<String>, has_file: bool, plus: &mut usize, minus: &mut usize) {
    if has_file {
        out.push(format!("  +{} -{}", *plus, *minus));
    }
    *plus = 0;
    *minus = 0;
}

fn summarize_git_push(text: &str) -> String {
    if contains_git_error(text) {
        return error_bearing_git_lines(text);
    }

    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.contains('%')
                && !trimmed.starts_with("Enumerating objects")
                && !trimmed.starts_with("Counting objects")
                && !trimmed.starts_with("Compressing objects")
                && !trimmed.starts_with("Writing objects")
        })
        .collect();
    if kept
        .iter()
        .any(|line| line.contains("Everything up-to-date"))
    {
        "ok (up-to-date)".to_string()
    } else if let Some(line) = kept.iter().find(|line| line.contains(" -> ")) {
        format!("ok {}", line.trim())
    } else if let Some(line) = kept.last() {
        line.trim().to_string()
    } else {
        String::new()
    }
}

fn summarize_git_pull(text: &str) -> String {
    if contains_git_error(text) {
        return error_bearing_git_lines(text);
    }

    if let Some((files, insertions, deletions)) =
        text.lines().find_map(|line| parse_shortstat(line.trim()))
    {
        format!("ok {files} files +{insertions} -{deletions}")
    } else if text.contains("Already up to date.") || text.contains("Already up-to-date.") {
        "ok (up-to-date)".to_string()
    } else {
        summarize_git_push(text)
    }
}

fn contains_git_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("fatal:") || lower.contains("error:")
}

fn error_bearing_git_lines(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start().to_ascii_lowercase();
            trimmed.starts_with("error:")
                || trimmed.starts_with("fatal:")
                || trimmed.starts_with("remote: error:")
                || trimmed.starts_with("remote: fatal:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_git_branch(text: &str) -> String {
    if contains_git_error(text) {
        return filter_lines(text, meaningful_git_line);
    }
    let branches: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let current = branches
        .iter()
        .find(|line| line.trim_start().starts_with('*'))
        .map(|line| line.trim().trim_start_matches("* "))
        .unwrap_or("none");
    format!("{} branches; current {current}", branches.len())
}

fn summarize_git_stash(text: &str) -> String {
    if contains_git_error(text) {
        return filter_lines(text, meaningful_git_line);
    }
    let stashes: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("stash@{"))
        .collect();
    if stashes.is_empty() {
        return filter_lines(text, meaningful_git_line);
    }
    let mut out = vec![format!("{} stashes", stashes.len())];
    out.extend(stashes.into_iter().take(5).map(str::to_string));
    out.join("\n")
}

fn summarize_search(text: &str) -> String {
    let rows = table_rows(text);
    if !rows.is_empty() {
        return rows
            .into_iter()
            .take(40)
            .map(|row| row.join(" | "))
            .collect::<Vec<_>>()
            .join("\n");
    }
    summarize_text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_extracts_failure_lines() {
        let candidate = compress_kind(
            DetectedKind::CargoTest,
            "running 1 test\ntest foo ... FAILED\nfailures:\n",
            "",
        );
        assert!(candidate.stdout.contains("FAILED"));
        assert!(
            candidate
                .strategy
                .contains(&"test-failure-focus".to_string())
        );
    }
}
