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

#[cfg(test)]
pub fn dedup_lines(text: &str) -> String {
    dedup_by_key(text, |line| line.to_string())
}

pub fn dedup_log_lines(text: &str) -> String {
    let filtered = text
        .lines()
        .filter(|line| !is_progress_noise(line))
        .collect::<Vec<_>>()
        .join("\n");
    let deduped = dedup_by_key(&filtered, normalize_log_line);
    let errors = text
        .lines()
        .filter(|line| is_error_line(line) && !deduped.contains(*line))
        .take(20)
        .map(|line| format!("ERROR excerpt: {line}"))
        .collect::<Vec<_>>();
    if errors.is_empty() {
        deduped
    } else if deduped.is_empty() {
        errors.join("\n")
    } else {
        format!("{deduped}\n{}", errors.join("\n"))
    }
}

fn dedup_by_key(text: &str, key: impl Fn(&str) -> String) -> String {
    let mut out = Vec::new();
    let mut prev_line: Option<String> = None;
    let mut prev_key: Option<String> = None;
    let mut count = 0usize;
    for line in text.lines() {
        let line_key = key(line);
        if prev_key.as_deref() == Some(line_key.as_str()) {
            count += 1;
            continue;
        }
        if let Some(p) = prev_line.as_deref() {
            push_dedup(&mut out, p, count);
        }
        prev_line = Some(line.to_string());
        prev_key = Some(line_key);
        count = 1;
    }
    if let Some(p) = prev_line.as_deref() {
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

pub fn list_summary(text: &str) -> String {
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for raw in text.lines().filter(|line| !line.trim().is_empty()) {
        let cleaned = raw.trim().trim_end_matches(':');
        let (dir, name) = split_path_for_summary(cleaned);
        groups.entry(dir).or_default().push(name);
    }
    if groups.is_empty() {
        return String::new();
    }
    let mut out = Vec::new();
    for (dir, mut names) in groups.into_iter().take(40) {
        names.sort();
        let total = names.len();
        let shown = names.into_iter().take(8).collect::<Vec<_>>().join(", ");
        let omitted = total.saturating_sub(8);
        if omitted == 0 {
            out.push(format!("{dir}: {shown}"));
        } else {
            out.push(format!("{dir}: {shown} ... ({omitted} omitted)"));
        }
    }
    out.join("\n")
}

pub fn search_summary(text: &str) -> String {
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (file, representative) = split_search_line(line);
        groups.entry(file).or_default().push(representative);
    }
    if groups.is_empty() {
        return String::new();
    }
    groups
        .into_iter()
        .take(40)
        .map(|(file, matches)| {
            let total = matches.len();
            let reps = matches
                .into_iter()
                .take(3)
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{file}: {total} match(es)\n{reps}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn text_file_summary(text: &str) -> String {
    let mut summary = bounded_summary(text, 8, 8, 1600);
    if looks_like_code(text) {
        let markers = text
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("def ")
                    || trimmed.starts_with("function ")
            })
            .take(20)
            .collect::<Vec<_>>();
        if !markers.is_empty() {
            summary = format!("code shape:\n{}\n---\n{summary}", markers.join("\n"));
        }
    }
    summary
}

pub fn env_summary(text: &str) -> String {
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut secret_entries = Vec::new();
    for line in text.lines().filter(|line| line.contains('=')) {
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let prefix = key.split('_').next().unwrap_or(key).to_string();
        let secret = is_secret_key(key);
        let shown_value = if secret { "***" } else { value };
        let entry = format!("{key}={shown_value}");
        if secret {
            secret_entries.push(entry.clone());
        }
        groups.entry(prefix).or_default().push(entry);
    }
    let mut out = Vec::new();
    if !secret_entries.is_empty() {
        out.push(format!(
            "secrets: {}",
            secret_entries
                .into_iter()
                .take(20)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.extend(groups.into_iter().take(30).map(|(prefix, vars)| {
        let total = vars.len();
        let shown = vars.into_iter().take(6).collect::<Vec<_>>().join(", ");
        let omitted = total.saturating_sub(6);
        if omitted == 0 {
            format!("{prefix}: {shown}")
        } else {
            format!("{prefix}: {shown} ... ({omitted} omitted)")
        }
    }));
    out.join("\n")
}

#[cfg(test)]
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
        Err(_) => {
            let shapes = text
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .take(20)
                .map(|value| json_shape(&value))
                .collect::<Vec<_>>();
            if shapes.is_empty() {
                summarize_text(text)
            } else {
                format!(
                    "ndjson rows={} sample_shapes:\n{}",
                    text.lines().count(),
                    shapes.join("\n")
                )
            }
        }
    }
}

fn json_shape(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let fields = map
                .iter()
                .take(20)
                .map(|(key, value)| format!("{key}:{}", json_type(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("object keys={} [{fields}]", map.len())
        }
        serde_json::Value::Array(items) => {
            let sample = items
                .first()
                .map(json_shape)
                .unwrap_or_else(|| "empty".to_string());
            format!("array len={} item_shape=({sample})", items.len())
        }
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

fn json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Null => "null",
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

fn split_path_for_summary(path: &str) -> (String, String) {
    let path = path.trim_start_matches("./");
    match path.rsplit_once('/') {
        Some((dir, name)) if !name.is_empty() => (dir.to_string(), name.to_string()),
        _ => (".".to_string(), path.to_string()),
    }
}

fn split_search_line(line: &str) -> (String, String) {
    if let Some((file, rest)) = line.split_once(':') {
        if let Some((line_no, text)) = rest.split_once(':')
            && line_no.chars().all(|c| c.is_ascii_digit())
        {
            return (file.to_string(), format!("{line_no}: {text}"));
        }
        return (file.to_string(), rest.to_string());
    }
    ("<unknown>".to_string(), line.to_string())
}

fn normalize_log_line(line: &str) -> String {
    let without_rfc3339 = regex::Regex::new(r"\d{4}-\d{2}-\d{2}[T ][0-9:.]+Z?")
        .expect("valid timestamp regex")
        .replace_all(line, "<ts>");
    regex::Regex::new(r"\b\d{2}:\d{2}:\d{2}(?:\.\d+)?\b")
        .expect("valid time regex")
        .replace_all(&without_rfc3339, "<time>")
        .to_string()
}

fn is_progress_noise(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("progress") && (lower.contains('%') || lower.contains("done"))
}

fn is_error_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error") || lower.contains("panic") || lower.contains("failed")
}

fn looks_like_code(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("def ")
            || trimmed.contains("{") && trimmed.contains("}")
    })
}

fn is_secret_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "secret",
        "token",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "credential",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
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
        let summary = json_shape_summary("{\"a\":1,\"b\":\"two\"}");
        assert!(summary.contains("keys=2"));
        assert!(summary.contains("a:number"));
        assert!(summary.contains("b:string"));
    }

    #[test]
    fn list_summary_groups_by_directory_and_caps() {
        let raw = (0..20)
            .map(|n| format!("src/module/file{n}.rs"))
            .collect::<Vec<_>>()
            .join("\n");
        let summary = list_summary(&raw);
        assert!(summary.contains("src/module:"));
        assert!(summary.contains("omitted"));
        assert!(summary.len() < raw.len());
    }

    #[test]
    fn search_summary_groups_by_file_with_line_numbers() {
        let raw = "src/a.rs:10:needle one\nsrc/a.rs:20:needle two\nsrc/b.rs:5:needle";
        let summary = search_summary(raw);
        assert!(summary.contains("src/a.rs: 2 match(es)"));
        assert!(summary.contains("10: needle one"));
    }

    #[test]
    fn text_summary_keeps_head_tail_and_code_shape() {
        let raw = format!("pub fn first() {{}}\n{}\nlast", "middle\n".repeat(50));
        let summary = text_file_summary(&raw);
        assert!(summary.contains("code shape"));
        assert!(summary.contains("pub fn first"));
        assert!(summary.contains("last"));
        assert!(summary.len() < raw.len());
    }

    #[test]
    fn log_dedup_normalizes_timestamps_and_keeps_errors() {
        let raw = "2026-01-01T00:00:00Z worker ok\n2026-01-01T00:00:01Z worker ok\nERROR failed\n";
        let summary = dedup_log_lines(raw);
        assert!(summary.contains("repeated 2x"));
        assert!(summary.contains("ERROR failed"));
    }

    #[test]
    fn json_shape_summarizes_arrays_and_ndjson() {
        let array = json_shape_summary("[{\"id\":1,\"name\":\"a\"},{\"id\":2,\"name\":\"b\"}]");
        assert!(array.contains("array len=2"));
        assert!(array.contains("id:number"));
        let ndjson = json_shape_summary("{\"id\":1}\n{\"id\":2}");
        assert!(ndjson.contains("ndjson rows=2"));
    }

    #[test]
    fn env_summary_masks_secret_values_and_groups_prefixes() {
        let summary = env_summary("AWS_TOKEN=abc\nAWS_REGION=us\nPATH=/bin");
        assert!(summary.contains("AWS_TOKEN=***"));
        assert!(!summary.contains("abc"));
        assert!(summary.contains("AWS:"));
    }
}
