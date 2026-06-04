#[derive(Debug, Clone)]
pub struct CompressionCandidate {
    pub stdout: String,
    pub stderr: String,
    pub omitted: bool,
    pub strategy: Vec<String>,
}

pub fn guard_expansion(
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

pub fn fallback_if_empty(compressed: String, raw: &str) -> String {
    if compressed.is_empty() && !raw.is_empty() {
        summarize_text(raw)
    } else {
        compressed
    }
}

pub fn summarize_text(text: &str) -> String {
    bounded_summary(text, 10, 10, 2000)
}

pub fn bounded_summary(
    text: &str,
    head_lines: usize,
    tail_lines: usize,
    max_bytes: usize,
) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= head_lines + tail_lines && text.len() <= max_bytes {
        return text.to_string();
    }
    let mut out = lines.iter().take(head_lines).copied().collect::<Vec<_>>();
    out.push("... omitted middle ...");
    out.extend(lines.iter().rev().take(tail_lines).rev().copied());
    out.join("\n")
}

pub fn dedup_lines(text: &str) -> String {
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

pub fn diagnostic_blocks(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("error") || lower.contains("panic") || lower.contains("traceback") {
            let start = idx.saturating_sub(1);
            let end = (idx + 3).min(lines.len());
            blocks.push(lines[start..end].join("\n"));
        }
    }
    blocks
}

pub fn table_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|line| line.contains('|'))
        .map(|line| {
            line.split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|row| row.len() > 1)
        .collect()
}

pub fn json_shape_summary(text: &str) -> String {
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

pub fn looks_like_json(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

pub fn has_repeated_adjacent_lines(text: &str) -> bool {
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

    fn data(stdout: &str, stderr: &str) -> crate::schema::CompressionData {
        crate::schema::CompressionData {
            mode: "summary".to_string(),
            applied: true,
            detected_kind: "summary".to_string(),
            stdout_compressed_bytes: stdout.len() as u64,
            stderr_compressed_bytes: stderr.len() as u64,
            stdout_original_bytes: 0,
            stderr_original_bytes: 0,
            omitted: false,
            strategy: vec!["test".to_string()],
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }
    }

    #[test]
    fn guard_rejects_stdout_expansion() {
        let guarded = guard_expansion(data("same", ""), "same", "");
        assert!(!guarded.applied);
        assert_eq!(guarded.strategy, vec!["expansion-guard"]);
    }

    #[test]
    fn guard_rejects_stderr_expansion() {
        let guarded = guard_expansion(data("", "same"), "", "same");
        assert!(!guarded.applied);
        assert_eq!(guarded.stderr_compressed_bytes, 0);
    }

    #[test]
    fn guard_rejects_mixed_stream_expansion() {
        let guarded = guard_expansion(data("ok", "longer"), "long", "short");
        assert!(!guarded.applied);
        assert_eq!(guarded.stdout, "");
        assert_eq!(guarded.stderr, "");
    }

    #[test]
    fn bounded_summary_keeps_head_and_tail() {
        let text = (0..30)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let summary = bounded_summary(&text, 2, 2, 20);
        assert!(summary.contains("line 0"));
        assert!(summary.contains("line 29"));
        assert!(summary.contains("omitted middle"));
    }

    #[test]
    fn line_deduplication_collapses_adjacent_repeats() {
        assert_eq!(dedup_lines("a\na\nb"), "a (repeated 2x)\nb");
    }

    #[test]
    fn diagnostic_extraction_includes_context() {
        let blocks = diagnostic_blocks("before\nerror: bad\nafter");
        assert_eq!(blocks[0], "before\nerror: bad\nafter");
    }

    #[test]
    fn table_parser_extracts_cells() {
        let rows = table_rows("| name | value |\n| a | 1 |");
        assert_eq!(rows[0], vec!["name", "value"]);
    }

    #[test]
    fn json_shape_extracts_object_keys() {
        assert!(json_shape_summary("{\"a\":1,\"b\":2}").contains("keys=2"));
    }
}
