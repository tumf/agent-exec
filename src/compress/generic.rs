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
        DetectedKind::Logs | DetectedKind::DockerLogs | DetectedKind::KubernetesLogs => (
            dedup_lines(raw_stdout),
            dedup_lines(raw_stderr),
            vec!["dedupe-repeated-lines".to_string()],
        ),
        DetectedKind::DockerTable => (
            summarize_table(raw_stdout, DOCKER_COLUMNS, ABNORMAL_WORDS),
            summarize_table(raw_stderr, DOCKER_COLUMNS, ABNORMAL_WORDS),
            vec!["docker-table-summary".to_string()],
        ),
        DetectedKind::KubernetesTable => (
            summarize_table(raw_stdout, KUBERNETES_COLUMNS, ABNORMAL_WORDS),
            summarize_table(raw_stderr, KUBERNETES_COLUMNS, ABNORMAL_WORDS),
            vec!["kubernetes-table-summary".to_string()],
        ),
        DetectedKind::GitHubCli | DetectedKind::GitLabCli => (
            summarize_issue_cli(raw_stdout),
            summarize_issue_cli(raw_stderr),
            vec![format!("{}-summary", kind.as_str())],
        ),
        DetectedKind::Aws => (
            summarize_aws(raw_stdout),
            summarize_aws(raw_stderr),
            vec!["aws-safe-summary".to_string()],
        ),
        DetectedKind::HttpTransfer => (
            summarize_http_transfer(raw_stdout),
            summarize_http_transfer(raw_stderr),
            vec!["http-progress-filter".to_string()],
        ),
        DetectedKind::PsqlTable => (
            summarize_table(raw_stdout, PSQL_COLUMNS, ABNORMAL_WORDS),
            summarize_table(raw_stderr, PSQL_COLUMNS, ABNORMAL_WORDS),
            vec!["psql-table-summary".to_string()],
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

const DOCKER_COLUMNS: &[&str] = &[
    "container id",
    "name",
    "names",
    "image",
    "status",
    "state",
    "ports",
    "service",
];
const KUBERNETES_COLUMNS: &[&str] = &[
    "namespace",
    "name",
    "ready",
    "status",
    "restarts",
    "age",
    "type",
    "cluster-ip",
    "external-ip",
];
const PSQL_COLUMNS: &[&str] = &[
    "id", "name", "status", "state", "created", "updated", "error",
];
const ABNORMAL_WORDS: &[&str] = &[
    "error",
    "fail",
    "crash",
    "backoff",
    "unhealthy",
    "exited",
    "pending",
    "terminating",
];

fn summarize_table(text: &str, preferred_columns: &[&str], abnormal_words: &[&str]) -> String {
    let Some((headers, rows)) = parse_table(text) else {
        return summarize_text(text);
    };
    let keep = selected_column_indexes(&headers, preferred_columns);
    if keep.is_empty() {
        return summarize_text(text);
    }

    let mut prioritized = rows;
    prioritized.sort_by_key(|row| !row_has_any(row, abnormal_words));
    let mut out = Vec::new();
    out.push(project_row(&headers, &keep));
    out.extend(
        prioritized
            .iter()
            .take(30)
            .map(|row| project_row(row, &keep)),
    );
    if prioritized.len() > 30 {
        out.push(format!("... +{} rows omitted", prioritized.len() - 30));
    }
    out.join("\n")
}

fn parse_table(text: &str) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() < 2 {
        return None;
    }
    if lines[0].contains('|') {
        let rows: Vec<Vec<String>> = lines
            .iter()
            .filter(|line| !line.trim().starts_with("+-") && !line.trim().starts_with("|-"))
            .map(|line| {
                line.split('|')
                    .map(str::trim)
                    .filter(|cell| !cell.is_empty() && !cell.chars().all(|c| c == '-'))
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|row| row.len() > 1)
            .collect();
        return split_header_rows(rows);
    }

    let (header, spans) = parse_fixed_width_header(lines[0]);
    if header.len() < 2 || !header.iter().any(|cell| is_known_header(cell)) {
        return None;
    }
    let rows = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim_start().starts_with('('))
        .map(|line| split_fixed_width_row(line, &spans))
        .filter(|row| row.len() == header.len())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        None
    } else {
        Some((header, rows))
    }
}

fn split_header_rows(rows: Vec<Vec<String>>) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let mut iter = rows.into_iter();
    let header = iter.next()?;
    let rows = iter.collect::<Vec<_>>();
    if rows.is_empty() {
        None
    } else {
        Some((header, rows))
    }
}

fn parse_fixed_width_header(line: &str) -> (Vec<String>, Vec<(usize, usize)>) {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }
        let start = idx;
        while idx < bytes.len() {
            if idx + 1 < bytes.len()
                && bytes[idx].is_ascii_whitespace()
                && bytes[idx + 1].is_ascii_whitespace()
            {
                break;
            }
            idx += 1;
        }
        let end = idx;
        spans.push((start, end));
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
    }
    let headers = spans
        .iter()
        .map(|(start, end)| line[*start..*end].trim().to_string())
        .collect();
    (headers, spans)
}

fn split_fixed_width_row(line: &str, spans: &[(usize, usize)]) -> Vec<String> {
    spans
        .iter()
        .enumerate()
        .map(|(idx, (start, _end))| {
            let end = spans
                .get(idx + 1)
                .map(|(next_start, _)| *next_start)
                .unwrap_or(line.len())
                .min(line.len());
            if *start >= line.len() {
                String::new()
            } else {
                line[*start..end].trim().to_string()
            }
        })
        .collect()
}

fn selected_column_indexes(headers: &[String], preferred_columns: &[&str]) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .filter_map(|(idx, header)| {
            let normalized = normalize_header(header);
            preferred_columns
                .iter()
                .any(|preferred| normalize_header(preferred) == normalized)
                .then_some(idx)
        })
        .collect()
}

fn normalize_header(header: &str) -> String {
    header
        .trim()
        .trim_matches('-')
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn is_known_header(header: &str) -> bool {
    let normalized = normalize_header(header);
    DOCKER_COLUMNS
        .iter()
        .chain(KUBERNETES_COLUMNS)
        .chain(PSQL_COLUMNS)
        .any(|known| normalize_header(known) == normalized)
}

fn project_row(row: &[String], keep: &[usize]) -> String {
    keep.iter()
        .filter_map(|idx| row.get(*idx))
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ")
}

fn row_has_any(row: &[String], words: &[&str]) -> bool {
    let lower = row.join(" ").to_ascii_lowercase();
    words.iter().any(|word| lower.contains(word))
}

fn summarize_issue_cli(text: &str) -> String {
    let keep_prefixes = [
        "title:",
        "state:",
        "author:",
        "labels:",
        "assignees:",
        "checks:",
        "status:",
        "url:",
    ];
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let keep = trimmed.starts_with('#')
            || lower.starts_with('!')
            || keep_prefixes.iter().any(|prefix| lower.starts_with(prefix))
            || lower.contains("failure")
            || lower.contains("failed")
            || lower.contains("error")
            || lower.contains("passed")
            || lower.contains("success");
        if keep {
            out.push(trimmed.to_string());
        }
        if out.len() >= 80 {
            break;
        }
    }
    fallback_if_empty(out.join("\n"), text)
}

fn summarize_aws(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return summarize_json_value(&value, 0).join("\n");
    }
    summarize_table(text, AWS_COLUMNS, ABNORMAL_WORDS)
}

const AWS_COLUMNS: &[&str] = &[
    "name",
    "id",
    "arn",
    "state",
    "status",
    "status-code",
    "instanceid",
    "functionname",
    "stackname",
    "resourcestatus",
    "eventid",
];

fn summarize_json_value(value: &serde_json::Value, depth: usize) -> Vec<String> {
    if depth > 2 {
        return vec!["... nested value omitted".to_string()];
    }
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(key, value)| !is_sensitive_or_large_key(key, value))
            .flat_map(|(key, value)| summarize_json_field(key, value, depth))
            .take(80)
            .collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .take(20)
            .enumerate()
            .flat_map(|(idx, item)| {
                summarize_json_value(item, depth + 1)
                    .into_iter()
                    .map(move |line| format!("[{idx}] {line}"))
            })
            .collect(),
        scalar => vec![scalar.to_string()],
    }
}

fn summarize_json_field(key: &str, value: &serde_json::Value, depth: usize) -> Vec<String> {
    if value.is_object() || value.is_array() {
        summarize_json_value(value, depth + 1)
            .into_iter()
            .map(|line| format!("{key}.{line}"))
            .collect()
    } else {
        vec![format!("{key}: {value}")]
    }
}

fn is_sensitive_or_large_key(key: &str, value: &serde_json::Value) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("policy")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
        || value.to_string().len() > 600
}

fn summarize_http_transfer(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('%')
                && !trimmed.contains("--:--:--")
                && !trimmed.contains("====>")
                && !trimmed.contains("#")
        })
        .filter(|line| !line.trim().is_empty())
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
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

    #[test]
    fn docker_table_keeps_key_columns_and_abnormal_rows_first() {
        let raw = "CONTAINER ID   IMAGE   COMMAND   CREATED   STATUS                     PORTS     NAMES\nabc            app:v1  run       1h        Up 1 hour                  80/tcp    web\ndef            db:v1   run       2h        Exited (1) 5 minutes ago   5432/tcp  db";
        let candidate = compress_kind(DetectedKind::DockerTable, raw, "");
        assert!(candidate.stdout.contains("IMAGE"));
        assert!(candidate.stdout.contains("STATUS"));
        assert!(candidate.stdout.contains("PORTS"));
        assert!(candidate.stdout.contains("NAMES"));
        assert!(
            candidate
                .stdout
                .lines()
                .nth(1)
                .unwrap_or("")
                .contains("Exited")
        );
        assert!(!candidate.stdout.contains("COMMAND"));
    }

    #[test]
    fn kubernetes_table_keeps_readiness_status_and_restarts() {
        let raw = "NAME        READY   STATUS             RESTARTS   AGE\napi-1       1/1     Running            0          1h\nworker-1    0/1     CrashLoopBackOff   4          5m";
        let candidate = compress_kind(DetectedKind::KubernetesTable, raw, "");
        assert!(
            candidate
                .stdout
                .contains("NAME | READY | STATUS | RESTARTS | AGE")
        );
        assert!(
            candidate
                .stdout
                .lines()
                .nth(1)
                .unwrap_or("")
                .contains("CrashLoopBackOff")
        );
    }

    #[test]
    fn logs_routes_deduplicate_repeated_container_logs() {
        let raw = "ok\nerror: failed to connect\nerror: failed to connect\n";
        let candidate = compress_kind(DetectedKind::KubernetesLogs, raw, "");
        assert!(
            candidate
                .stdout
                .contains("error: failed to connect (repeated 2x)")
        );
    }

    #[test]
    fn gh_glab_summary_keeps_identity_state_checks_and_bounded_body() {
        let raw = "title: Add feature\nstate: OPEN\nlabels: bug,ci\nchecks: failing\n# Body\nDetails\nnoise\nerror: check failed";
        let candidate = compress_kind(DetectedKind::GitHubCli, raw, "");
        assert!(candidate.stdout.contains("title: Add feature"));
        assert!(candidate.stdout.contains("checks: failing"));
        assert!(candidate.stdout.contains("# Body"));
        assert!(candidate.stdout.contains("error: check failed"));
    }

    #[test]
    fn aws_json_omits_policies_secrets_and_keeps_resource_status() {
        let raw = r#"{"UserId":"AID","Account":"123","SecretAccessKey":"nope","PolicyDocument":{"Statement":[{"Effect":"Allow"}]},"Functions":[{"FunctionName":"fn","State":"Active","LastUpdateStatus":"Successful"}]}"#;
        let candidate = compress_kind(DetectedKind::Aws, raw, "");
        assert!(candidate.stdout.contains("UserId"));
        assert!(candidate.stdout.contains("FunctionName"));
        assert!(candidate.stdout.contains("State"));
        assert!(!candidate.stdout.contains("PolicyDocument"));
        assert!(!candidate.stdout.contains("nope"));
    }

    #[test]
    fn curl_wget_progress_is_stripped_but_result_context_remains() {
        let raw = "  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\n100 1024  100 1024    0     0   10k      0 --:--:-- --:--:-- --:--:-- 10k\nHTTP/2 200\nserver: example";
        let candidate = compress_kind(DetectedKind::HttpTransfer, raw, "");
        assert!(candidate.stdout.contains("HTTP/2 200"));
        assert!(!candidate.stdout.contains("--:--:--"));
        assert!(!candidate.stdout.contains("% Total"));
    }

    #[test]
    fn psql_table_keeps_identity_and_status_columns() {
        let raw = " id | name | status | policy_document\n----+------+--------+----------------\n 1  | app  | ok     | very-large";
        let candidate = compress_kind(DetectedKind::PsqlTable, raw, "");
        assert!(candidate.stdout.contains("id | name | status"));
        assert!(!candidate.stdout.contains("policy_document"));
    }
}
