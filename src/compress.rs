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
    Git,
    Json,
    Summary,
}

impl DetectedKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Tests => "tests",
            Self::Logs => "logs",
            Self::Git => "git",
            Self::Json => "json",
            Self::Summary => "summary",
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
        DetectedKind::Git => (
            summarize_git(input.stdout),
            summarize_git(input.stderr),
            vec!["git-summary".to_string()],
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
    if stdout.len() < input.stdout.len() || stderr.len() < input.stderr.len() {
        strategy.push("truncation".to_string());
    }

    Some(crate::schema::CompressionData {
        mode: input.mode.as_str().to_string(),
        applied: true,
        detected_kind: kind.as_str().to_string(),
        stdout_compressed_bytes: stdout.len() as u64,
        stderr_compressed_bytes: stderr.len() as u64,
        stdout_original_bytes: input.stdout_original_bytes,
        stderr_original_bytes: input.stderr_original_bytes,
        omitted: stdout.len() < input.stdout.len() || stderr.len() < input.stderr.len(),
        strategy,
        stdout,
        stderr,
    })
}

fn mode_kind(mode: CompressionMode) -> DetectedKind {
    match mode {
        CompressionMode::Off | CompressionMode::Route => DetectedKind::Summary,
        CompressionMode::Errors => DetectedKind::Errors,
        CompressionMode::Tests => DetectedKind::Tests,
        CompressionMode::Logs => DetectedKind::Logs,
        CompressionMode::Git => DetectedKind::Git,
        CompressionMode::Json => DetectedKind::Json,
        CompressionMode::Summary => DetectedKind::Summary,
    }
}

fn detect_kind(command: &[String], stdout: &str, stderr: &str) -> DetectedKind {
    let command_text = command.join(" ").to_ascii_lowercase();
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if command_text.contains("git ") || command.first().is_some_and(|s| s == "git") {
        DetectedKind::Git
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

fn summarize_git(text: &str) -> String {
    filter_lines(text, |line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("diff --git")
            || trimmed.starts_with("modified:")
            || trimmed.starts_with("deleted:")
            || trimmed.starts_with("new file:")
            || trimmed.starts_with("+")
            || trimmed.starts_with("-")
            || trimmed.starts_with("commit ")
            || trimmed.contains("changed")
    })
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
        let data = compress(CompressionInput {
            command: &[],
            stdout: "same\nsame\nother\n",
            stderr: "",
            stdout_original_bytes: 16,
            stderr_original_bytes: 0,
            mode: CompressionMode::Logs,
        })
        .unwrap();
        assert!(data.stdout.contains("repeated 2x"));
    }
}
