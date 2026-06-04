use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum CompressionMode {
    Off,
    #[default]
    Route,
    Errors,
    Tests,
    Logs,
    Git,
    Json,
    Summary,
}

impl CompressionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Route => "route",
            Self::Errors => "errors",
            Self::Tests => "tests",
            Self::Logs => "logs",
            Self::Git => "git",
            Self::Json => "json",
            Self::Summary => "summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedKind {
    Errors,
    Tests,
    Logs,
    Git(GitKind),
    Json,
    Summary,
}

impl DetectedKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Tests => "tests",
            Self::Logs => "logs",
            Self::Git(kind) => kind.as_str(),
            Self::Json => "json",
            Self::Summary => "summary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitKind {
    Generic,
    Status,
    Log,
    Diff,
    Show,
    Push,
    Pull,
    Branch,
    Stash,
}

impl GitKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "git",
            Self::Status => "git-status",
            Self::Log => "git-log",
            Self::Diff => "git-diff",
            Self::Show => "git-show",
            Self::Push => "git-push",
            Self::Pull => "git-pull",
            Self::Branch => "git-branch",
            Self::Stash => "git-stash",
        }
    }
}

#[derive(Debug)]
pub struct CompressionInput<'a> {
    pub command: &'a [String],
    pub stdout: &'a str,
    pub stderr: &'a str,
    pub stdout_original_bytes: u64,
    pub stderr_original_bytes: u64,
    pub mode: CompressionMode,
}

pub fn resolve_cli_mode(
    compress: Option<CompressionMode>,
    rtk: Option<CompressionMode>,
) -> Result<Option<CompressionMode>, String> {
    match (compress, rtk) {
        (Some(a), Some(b)) if a != b => Err(format!(
            "--compress {} conflicts with --rtk {}",
            a.as_str(),
            b.as_str()
        )),
        (Some(a), _) | (_, Some(a)) => Ok(Some(a)),
        (None, None) => Ok(None),
    }
}

pub fn compress(input: CompressionInput<'_>) -> Option<crate::schema::CompressionData> {
    if input.mode == CompressionMode::Off {
        return None;
    }

    let kind = if input.mode == CompressionMode::Route {
        detect_kind(input.command, input.stdout, input.stderr)
    } else {
        mode_kind(input.mode)
    };

    let (stdout, stderr, mut strategy) = match kind {
        DetectedKind::Errors => (
            extract_error_lines(input.stdout),
            extract_error_lines(input.stderr),
            vec!["failure-focus".to_string()],
        ),
        DetectedKind::Tests => (
            extract_test_lines(input.stdout),
            extract_test_lines(input.stderr),
            vec!["test-failure-focus".to_string()],
        ),
        DetectedKind::Logs => (
            dedup_lines(input.stdout),
            dedup_lines(input.stderr),
            vec!["dedupe-repeated-lines".to_string()],
        ),
        DetectedKind::Git(git_kind) => (
            summarize_git(git_kind, input.stdout),
            summarize_git(git_kind, input.stderr),
            vec![format!("{}-summary", git_kind.as_str())],
        ),
        DetectedKind::Json => (
            summarize_json(input.stdout),
            summarize_json(input.stderr),
            vec!["json-structure".to_string()],
        ),
        DetectedKind::Summary => (
            summarize_text(input.stdout),
            summarize_text(input.stderr),
            vec!["bounded-summary".to_string()],
        ),
    };

    let stdout = fallback_if_empty(stdout, input.stdout);
    let stderr = fallback_if_empty(stderr, input.stderr);
    let omitted = stdout.len() < input.stdout.len() || stderr.len() < input.stderr.len();
    if omitted {
        strategy.push("truncation".to_string());
    }

    Some(guard_expansion(
        crate::schema::CompressionData {
            mode: input.mode.as_str().to_string(),
            applied: true,
            detected_kind: kind.as_str().to_string(),
            stdout_compressed_bytes: stdout.len() as u64,
            stderr_compressed_bytes: stderr.len() as u64,
            stdout_original_bytes: input.stdout_original_bytes,
            stderr_original_bytes: input.stderr_original_bytes,
            omitted,
            strategy,
            stdout,
            stderr,
        },
        input.stdout,
        input.stderr,
    ))
}

fn guard_expansion(
    mut data: crate::schema::CompressionData,
    raw_stdout: &str,
    raw_stderr: &str,
) -> crate::schema::CompressionData {
    let stdout_expands = !raw_stdout.is_empty() && data.stdout.len() >= raw_stdout.len();
    let stderr_expands = !raw_stderr.is_empty() && data.stderr.len() >= raw_stderr.len();
    if stdout_expands || stderr_expands {
        data.applied = false;
        data.stdout.clear();
        data.stderr.clear();
        data.stdout_compressed_bytes = 0;
        data.stderr_compressed_bytes = 0;
        data.omitted = false;
        data.strategy.clear();
        data.strategy.push("expansion-guard".to_string());
    }
    data
}

fn mode_kind(mode: CompressionMode) -> DetectedKind {
    match mode {
        CompressionMode::Off | CompressionMode::Route => DetectedKind::Summary,
        CompressionMode::Errors => DetectedKind::Errors,
        CompressionMode::Tests => DetectedKind::Tests,
        CompressionMode::Logs => DetectedKind::Logs,
        CompressionMode::Git => DetectedKind::Git(GitKind::Generic),
        CompressionMode::Json => DetectedKind::Json,
        CompressionMode::Summary => DetectedKind::Summary,
    }
}

fn detect_kind(command: &[String], stdout: &str, stderr: &str) -> DetectedKind {
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if let Some(git_kind) = detect_git_kind(command) {
        DetectedKind::Git(git_kind)
    } else if looks_like_json(stdout) || looks_like_json(stderr) {
        DetectedKind::Json
    } else if text.contains("test") && (text.contains("fail") || text.contains("passed")) {
        DetectedKind::Tests
    } else if text.contains("error") || text.contains("panic") || text.contains("traceback") {
        DetectedKind::Errors
    } else if has_repeated_adjacent_lines(stdout) || has_repeated_adjacent_lines(stderr) {
        DetectedKind::Logs
    } else {
        DetectedKind::Summary
    }
}

pub fn detect_git_kind(command: &[String]) -> Option<GitKind> {
    let git_index = command.iter().position(|part| part == "git")?;
    let mut skip_value = false;
    let subcommand = command.iter().skip(git_index + 1).find(|part| {
        if skip_value {
            skip_value = false;
            return false;
        }
        if matches!(part.as_str(), "-C" | "-c" | "--git-dir" | "--work-tree") {
            skip_value = true;
            return false;
        }
        !part.starts_with('-') && !part.contains('=')
    })?;
    Some(match subcommand.as_str() {
        "status" => GitKind::Status,
        "log" => GitKind::Log,
        "diff" => GitKind::Diff,
        "show" => GitKind::Show,
        "push" => GitKind::Push,
        "pull" => GitKind::Pull,
        "branch" => GitKind::Branch,
        "stash" => GitKind::Stash,
        _ => GitKind::Generic,
    })
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
    filter_lines(text, |line| {
        let lower = line.to_ascii_lowercase();
        keywords.iter().any(|k| lower.contains(k))
    })
}

fn extract_test_lines(text: &str) -> String {
    filter_lines(text, |line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("test")
            || lower.contains("fail")
            || lower.contains("passed")
            || lower.contains("FAILED")
    })
}

fn filter_lines(text: &str, keep: impl Fn(&str) -> bool) -> String {
    text.lines()
        .filter(|line| keep(line))
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
}

fn dedup_lines(text: &str) -> String {
    let mut out = Vec::new();
    let mut prev: Option<&str> = None;
    let mut count = 0usize;
    for line in text.lines() {
        if Some(line) == prev {
            count += 1;
            continue;
        }
        if let Some(p) = prev {
            push_dedup(&mut out, p, count);
        }
        prev = Some(line);
        count = 1;
    }
    if let Some(p) = prev {
        push_dedup(&mut out, p, count);
    }
    out.join("\n")
}

fn push_dedup(out: &mut Vec<String>, line: &str, count: usize) {
    if count > 1 {
        out.push(format!("{line} (repeated {count}x)"));
    } else {
        out.push(line.to_string());
    }
}

fn summarize_git(kind: GitKind, text: &str) -> String {
    match kind {
        GitKind::Status => summarize_git_status(text),
        GitKind::Log => summarize_git_log(text),
        GitKind::Diff | GitKind::Show => summarize_git_diff(text),
        GitKind::Push => summarize_git_push(text),
        GitKind::Pull => summarize_git_pull(text),
        GitKind::Branch => summarize_git_branch(text),
        GitKind::Stash => summarize_git_stash(text),
        GitKind::Generic => summarize_git_generic(text),
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
        && !trimmed.starts_with("(")
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

fn summarize_json(text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    match serde_json::from_str::<serde_json::Value>(text.trim()) {
        Ok(value) => json_shape(&value),
        Err(_) => summarize_text(text),
    }
}

fn json_shape(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => format!(
            "object keys={} [{}]",
            map.len(),
            map.keys().take(20).cloned().collect::<Vec<_>>().join(", ")
        ),
        serde_json::Value::Array(items) => format!("array len={}", items.len()),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

fn summarize_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 20 && text.len() <= 2000 {
        return text.to_string();
    }
    let mut out = lines.iter().take(10).copied().collect::<Vec<_>>();
    out.push("... omitted middle ...");
    out.extend(lines.iter().rev().take(10).rev().copied());
    out.join("\n")
}

fn fallback_if_empty(compressed: String, raw: &str) -> String {
    if compressed.is_empty() && !raw.is_empty() {
        summarize_text(raw)
    } else {
        compressed
    }
}

fn looks_like_json(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn has_repeated_adjacent_lines(text: &str) -> bool {
    let mut prev = None;
    for line in text.lines() {
        if Some(line) == prev {
            return true;
        }
        prev = Some(line);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_cli_modes_are_rejected() {
        let err = resolve_cli_mode(Some(CompressionMode::Errors), Some(CompressionMode::Logs))
            .unwrap_err();
        assert!(err.contains("conflicts"));
    }

    #[test]
    fn logs_mode_deduplicates_lines() {
        let raw = format!("{}other\n", "same\n".repeat(20));
        let data = compress(CompressionInput {
            command: &[],
            stdout: &raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Logs,
        })
        .unwrap();
        assert!(data.applied);
        assert!(data.stdout.contains("repeated 20x"));
        assert!(data.stdout.len() < raw.len());
    }

    #[test]
    fn expansion_guard_suppresses_larger_candidate() {
        let raw = "same\nsame\nother\n";
        let data = compress(CompressionInput {
            command: &[],
            stdout: raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Logs,
        })
        .unwrap();
        assert!(!data.applied);
        assert_eq!(data.stdout, "");
        assert_eq!(data.stderr, "");
        assert_eq!(data.stdout_compressed_bytes, 0);
        assert_eq!(data.stderr_compressed_bytes, 0);
        assert_eq!(data.strategy, vec!["expansion-guard"]);
    }

    #[test]
    fn expansion_guard_suppresses_equal_size_candidate() {
        let raw = "short";
        let data = compress(CompressionInput {
            command: &[],
            stdout: raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Summary,
        })
        .unwrap();
        assert!(!data.applied);
        assert_eq!(data.stdout, "");
        assert_eq!(data.strategy, vec!["expansion-guard"]);
    }

    #[test]
    fn expansion_guard_allows_smaller_candidate() {
        let raw = format!("{}tail\n", "line\n".repeat(50));
        let data = compress(CompressionInput {
            command: &[],
            stdout: &raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Summary,
        })
        .unwrap();
        assert!(data.applied);
        assert!(data.stdout.len() < raw.len());
        assert!(data.strategy.contains(&"truncation".to_string()));
    }

    #[test]
    fn expansion_guard_suppresses_when_one_stream_expands() {
        let stdout = "ok\n";
        let stderr = format!("{}tail\n", "line\n".repeat(50));
        let data = compress(CompressionInput {
            command: &[],
            stdout,
            stderr: &stderr,
            stdout_original_bytes: stdout.len() as u64,
            stderr_original_bytes: stderr.len() as u64,
            mode: CompressionMode::Summary,
        })
        .unwrap();
        assert!(!data.applied);
        assert_eq!(data.stdout, "");
        assert_eq!(data.stderr, "");
        assert_eq!(data.strategy, vec!["expansion-guard"]);
    }

    #[test]
    fn git_classifier_maps_representative_argv() {
        for (argv, expected) in [
            (vec!["git", "status"], GitKind::Status),
            (vec!["git", "log", "--stat"], GitKind::Log),
            (vec!["git", "diff"], GitKind::Diff),
            (vec!["git", "show"], GitKind::Show),
            (vec!["git", "push"], GitKind::Push),
            (vec!["git", "pull"], GitKind::Pull),
            (vec!["git", "branch"], GitKind::Branch),
            (vec!["git", "stash"], GitKind::Stash),
            (vec!["git", "-C", "repo", "status"], GitKind::Status),
        ] {
            let command = argv.into_iter().map(String::from).collect::<Vec<_>>();
            assert_eq!(detect_git_kind(&command), Some(expected));
        }
    }

    #[test]
    fn git_status_keeps_state_and_removes_hints() {
        let raw = "On branch main\nYour branch is up to date with 'origin/main'.\n\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n\tmodified:   src/lib.rs\n\n";
        let data = compress(CompressionInput {
            command: &["git".into(), "status".into()],
            stdout: raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Route,
        })
        .unwrap();
        assert!(data.stdout.contains("On branch main"));
        assert!(data.stdout.contains("modified:   src/lib.rs"));
        assert!(!data.stdout.contains("use \"git add"));
    }

    #[test]
    fn git_status_keeps_detached_and_rebase_state() {
        let raw = "interactive rebase in progress; onto abc1234\nHEAD detached at abc1234\n  (use \"git switch -\" to return)\n\n";
        let data = compress(CompressionInput {
            command: &["git".into(), "status".into()],
            stdout: raw,
            stderr: "",
            stdout_original_bytes: raw.len() as u64,
            stderr_original_bytes: 0,
            mode: CompressionMode::Route,
        })
        .unwrap();
        assert!(data.stdout.contains("interactive rebase in progress"));
        assert!(data.stdout.contains("HEAD detached"));
        assert!(!data.stdout.contains("git switch"));
    }

    #[test]
    fn git_push_pull_branch_and_stash_summarize() {
        let push = "Enumerating objects: 10, done.\nCounting objects: 100% (10/10), done.\nTo github.com:x/y.git\n   abc..def  main -> main\n";
        assert_eq!(
            summarize_git(GitKind::Push, push),
            "ok abc..def  main -> main"
        );
        assert_eq!(
            summarize_git(GitKind::Push, "Everything up-to-date\n"),
            "ok (up-to-date)"
        );
        assert_eq!(
            summarize_git(
                GitKind::Push,
                "Enumerating objects: 1, done.\nerror: failed to push some refs\nhint: Updates were rejected\nfatal: the remote end hung up unexpectedly\n"
            ),
            "error: failed to push some refs\nfatal: the remote end hung up unexpectedly"
        );
        assert_eq!(
            summarize_git(
                GitKind::Pull,
                "remote: Counting objects: 100% (1/1), done.\nerror: Your local changes would be overwritten by merge\nhint: Commit your changes\n"
            ),
            "error: Your local changes would be overwritten by merge"
        );
        assert_eq!(
            summarize_git(
                GitKind::Pull,
                "error: Your local changes would be overwritten by merge\n 2 files changed, 4 insertions(+), 1 deletion(-)\nfatal: merge failed\n"
            ),
            "error: Your local changes would be overwritten by merge\nfatal: merge failed"
        );
        assert_eq!(
            summarize_git(
                GitKind::Pull,
                "Fast-forward\n 2 files changed, 4 insertions(+), 1 deletion(-)\n"
            ),
            "ok 2 files +4 -1"
        );
        assert_eq!(
            summarize_git(GitKind::Branch, "  dev\n* main\n"),
            "2 branches; current main"
        );
        assert!(
            summarize_git(GitKind::Stash, "stash@{0}: WIP\nstash@{1}: WIP\n").contains("2 stashes")
        );
    }
}
