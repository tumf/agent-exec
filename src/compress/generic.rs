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
        DetectedKind::Git | DetectedKind::GitLog => (
            summarize_git(raw_stdout),
            summarize_git(raw_stderr),
            vec!["git-summary".to_string()],
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
    filter_lines(text, |line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("test")
            || lower.contains("fail")
            || lower.contains("passed")
            || lower.contains("failed")
    })
}

fn filter_lines(text: &str, keep: impl Fn(&str) -> bool) -> String {
    text.lines()
        .filter(|line| keep(line))
        .take(80)
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_git(text: &str) -> String {
    filter_lines(text, |line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("diff --git")
            || trimmed.starts_with("modified:")
            || trimmed.starts_with("deleted:")
            || trimmed.starts_with("new file:")
            || trimmed.starts_with('+')
            || trimmed.starts_with('-')
            || trimmed.starts_with("commit ")
            || trimmed.contains("changed")
    })
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
