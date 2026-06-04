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
    CargoTest,
    CargoBuild,
    TestRunner,
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
            Self::CargoTest => "cargo-test",
            Self::CargoBuild => "cargo-build",
            Self::TestRunner => "test-runner",
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
            compress_test_output(input.stdout),
            compress_test_output(input.stderr),
            vec!["test-failure-focus".to_string()],
        ),
        DetectedKind::CargoTest => (
            compress_cargo_test_output(input.stdout),
            compress_cargo_test_output(input.stderr),
            vec!["cargo-test-failure-focus".to_string()],
        ),
        DetectedKind::CargoBuild => (
            compress_rust_diagnostics(input.stdout),
            compress_rust_diagnostics(input.stderr),
            vec!["rust-diagnostic-focus".to_string()],
        ),
        DetectedKind::TestRunner => (
            compress_test_output(input.stdout),
            compress_test_output(input.stderr),
            vec!["generic-test-failure-focus".to_string()],
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
        CompressionMode::Git => DetectedKind::Git,
        CompressionMode::Json => DetectedKind::Json,
        CompressionMode::Summary => DetectedKind::Summary,
    }
}

fn detect_kind(command: &[String], stdout: &str, stderr: &str) -> DetectedKind {
    let command_text = command.join(" ").to_ascii_lowercase();
    let program = command.first().map(|s| s.as_str()).unwrap_or_default();
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();

    if program == "git" || command_text.contains("git ") {
        DetectedKind::Git
    } else if is_cargo_test_command(command) {
        DetectedKind::CargoTest
    } else if is_cargo_build_command(command) {
        DetectedKind::CargoBuild
    } else if is_generic_test_command(command) {
        DetectedKind::TestRunner
    } else if looks_like_json(stdout) || looks_like_json(stderr) {
        DetectedKind::Json
    } else if looks_like_rust_diagnostics(&text) {
        DetectedKind::CargoBuild
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

fn is_cargo_test_command(command: &[String]) -> bool {
    command.first().is_some_and(|s| s == "cargo") && command.iter().any(|s| s == "test")
}

fn is_cargo_build_command(command: &[String]) -> bool {
    command.first().is_some_and(|s| s == "cargo")
        && command
            .iter()
            .any(|s| matches!(s.as_str(), "build" | "check" | "clippy"))
}

fn is_generic_test_command(command: &[String]) -> bool {
    let first = command.first().map(|s| s.as_str()).unwrap_or_default();
    matches!(first, "pytest" | "vitest" | "jest")
        || (matches!(first, "npm" | "pnpm") && command.iter().any(|s| s == "test"))
        || (first == "go" && command.iter().any(|s| s == "test"))
}

fn looks_like_rust_diagnostics(text: &str) -> bool {
    text.contains("error[") || text.contains("warning[") || text.contains(" --> ")
}

fn compress_rust_diagnostics(text: &str) -> String {
    let mut out = Vec::new();
    let mut diagnostic_count = 0usize;
    let mut omitted_progress = 0usize;
    let mut in_block = false;
    let mut block_lines = 0usize;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if is_cargo_progress_line(trimmed) {
            omitted_progress += 1;
            continue;
        }
        if is_rust_diagnostic_start(trimmed) {
            diagnostic_count += 1;
            if diagnostic_count > 6 {
                continue;
            }
            in_block = true;
            block_lines = 0;
            out.push(line.to_string());
            continue;
        }
        if in_block {
            if is_block_continuation(trimmed) {
                if block_lines < 28 {
                    out.push(line.to_string());
                }
                block_lines += 1;
            } else if trimmed.is_empty() {
                if block_lines < 28 {
                    out.push(String::new());
                }
            } else {
                in_block = false;
            }
        }
        if !in_block && is_rust_summary_line(trimmed) {
            out.push(line.to_string());
        }
    }

    if diagnostic_count > 6 {
        out.push(format!(
            "... omitted {} additional diagnostics ...",
            diagnostic_count - 6
        ));
    }
    if omitted_progress > 0 {
        out.push(format!(
            "... omitted {omitted_progress} cargo progress lines ..."
        ));
    }
    out.join("\n")
}

fn is_cargo_progress_line(trimmed: &str) -> bool {
    [
        "Compiling ",
        "Checking ",
        "Finished ",
        "Running ",
        "Fresh ",
        "Blocking ",
        "Waiting ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn is_rust_diagnostic_start(trimmed: &str) -> bool {
    trimmed.starts_with("error[")
        || trimmed.starts_with("warning[")
        || trimmed.starts_with("error:")
        || trimmed.starts_with("warning:")
}

fn is_block_continuation(trimmed: &str) -> bool {
    trimmed.starts_with("-->")
        || trimmed.starts_with("|")
        || trimmed.starts_with("=")
        || trimmed.starts_with("note:")
        || trimmed.starts_with("help:")
        || trimmed.starts_with("::")
        || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
}

fn is_rust_summary_line(trimmed: &str) -> bool {
    trimmed.starts_with("error: could not compile")
        || trimmed.starts_with("error: aborting")
        || trimmed.starts_with("warning:") && trimmed.contains("warning emitted")
}

fn compress_cargo_test_output(text: &str) -> String {
    let mut out = compress_test_output(text);
    let diagnostics = compress_rust_diagnostics(text);
    if !diagnostics.is_empty() && !out.contains(&diagnostics) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&diagnostics);
    }
    out
}

fn compress_test_output(text: &str) -> String {
    let mut out = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut ignored = 0usize;
    let mut failed_names = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0usize;
    let mut backtrace_lines = 0usize;

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if is_skip_line(trimmed) {
            ignored += 1;
            continue;
        }
        if is_pass_line(trimmed) {
            passed += 1;
            continue;
        }
        if is_fail_line(trimmed) {
            failed += 1;
            if let Some(name) = extract_failure_name(trimmed) {
                failed_names.push(name);
            }
            out.push(line.to_string());
            in_failure = true;
            failure_lines = 0;
            continue;
        }
        if lower.contains("test result:")
            || lower.starts_with("failed ")
            || lower.contains(" failed,")
            || lower.contains(" passed")
        {
            out.push(line.to_string());
            continue;
        }
        if starts_failure_section(trimmed) {
            in_failure = true;
            failure_lines = 0;
            out.push(line.to_string());
            continue;
        }
        if is_panic_or_assertion_line(trimmed) {
            in_failure = true;
            if failure_lines < 80 {
                out.push(line.to_string());
            }
            failure_lines += 1;
            continue;
        }
        if in_failure {
            if is_stack_frame(trimmed) {
                if backtrace_lines < 12 {
                    out.push(line.to_string());
                }
                backtrace_lines += 1;
            } else if (failure_lines < 80 && should_keep_failure_context(trimmed))
                || (failure_lines < 12 && !trimmed.is_empty() && !is_cargo_progress_line(trimmed))
            {
                out.push(line.to_string());
            } else if trimmed.is_empty() && failure_lines < 80 {
                out.push(String::new());
            } else if is_pass_line(trimmed) || is_cargo_progress_line(trimmed) {
                in_failure = false;
            }
            failure_lines += 1;
        }
    }

    let mut header = Vec::new();
    if passed + failed + ignored > 0 {
        header.push(format!(
            "test summary: passed={passed} failed={failed} skipped={ignored}"
        ));
    }
    if !failed_names.is_empty() {
        header.push(format!("failed tests: {}", failed_names.join(", ")));
    }
    if backtrace_lines > 12 {
        out.push(format!(
            "... omitted {} backtrace frames ...",
            backtrace_lines - 12
        ));
    }
    header.extend(out);
    header.join("\n")
}

fn is_pass_line(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("test ") && (lower.ends_with(" ... ok") || lower.contains(" ok"))
        || lower.starts_with("pass ")
        || lower.starts_with("✓")
        || lower.starts_with("ok  ")
        || lower == "ok"
}

fn is_skip_line(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    lower.contains("ignored")
        || lower.contains("skipped")
        || lower.starts_with("skip ")
        || lower.starts_with("--- skip:")
}

fn is_fail_line(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    (lower.starts_with("test ") && (lower.ends_with(" ... failed") || lower.contains(" failed")))
        || lower.starts_with("fail ")
        || lower.starts_with("failed ")
        || lower.starts_with("--- fail:")
        || lower.contains(" failed") && lower.contains("::")
}

fn extract_failure_name(trimmed: &str) -> Option<String> {
    if let Some(rest) = trimmed.strip_prefix("test ") {
        return rest.split_whitespace().next().map(ToString::to_string);
    }
    if let Some(rest) = trimmed.strip_prefix("FAIL ") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("FAILED ") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("--- FAIL:") {
        return rest.split_whitespace().next().map(ToString::to_string);
    }
    None
}

fn starts_failure_section(trimmed: &str) -> bool {
    trimmed == "failures:"
        || trimmed.starts_with("---- ")
        || trimmed.starts_with("FAILURES")
        || trimmed.starts_with("FAILED TESTS")
}

fn is_panic_or_assertion_line(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    lower.contains("panicked at")
        || lower.contains("panic:")
        || lower.contains("assertion")
        || lower.contains("traceback")
        || lower.contains("expected") && lower.contains("actual")
        || lower.contains("thread '")
}

fn should_keep_failure_context(trimmed: &str) -> bool {
    let lower = trimmed.to_ascii_lowercase();
    !trimmed.is_empty()
        && (lower.contains("error")
            || lower.contains("failed")
            || lower.contains("failure")
            || lower.contains("panic")
            || lower.contains("assert")
            || lower.contains("expected")
            || lower.contains("actual")
            || lower.contains(" at ")
            || lower.contains(" --> ")
            || trimmed.starts_with('|')
            || trimmed.starts_with("left:")
            || trimmed.starts_with("right:"))
}

fn is_stack_frame(trimmed: &str) -> bool {
    trimmed.starts_with("at ")
        || trimmed.starts_with("File ")
        || trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() && trimmed.contains(':'))
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

    fn compressed_for(
        command: &[&str],
        stdout: &str,
        stderr: &str,
    ) -> crate::schema::CompressionData {
        let command = command.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        compress(CompressionInput {
            command: &command,
            stdout,
            stderr,
            stdout_original_bytes: stdout.len() as u64,
            stderr_original_bytes: stderr.len() as u64,
            mode: CompressionMode::Route,
        })
        .unwrap()
    }

    #[test]
    fn conflicting_cli_modes_are_rejected() {
        let err = resolve_cli_mode(Some(CompressionMode::Errors), Some(CompressionMode::Logs))
            .unwrap_err();
        assert!(err.contains("conflicts"));
    }

    #[test]
    fn classifier_detects_rust_and_common_test_commands() {
        assert_eq!(
            detect_kind(&["cargo".into(), "test".into()], "", "").as_str(),
            "cargo-test"
        );
        assert_eq!(
            detect_kind(&["cargo".into(), "build".into()], "", "").as_str(),
            "cargo-build"
        );
        assert_eq!(
            detect_kind(&["cargo".into(), "check".into()], "", "").as_str(),
            "cargo-build"
        );
        assert_eq!(
            detect_kind(&["cargo".into(), "clippy".into()], "", "").as_str(),
            "cargo-build"
        );
        assert_eq!(
            detect_kind(&["pytest".into()], "", "").as_str(),
            "test-runner"
        );
        assert_eq!(
            detect_kind(&["npm".into(), "test".into()], "", "").as_str(),
            "test-runner"
        );
        assert_eq!(
            detect_kind(&["go".into(), "test".into()], "", "").as_str(),
            "test-runner"
        );
    }

    #[test]
    fn rust_diagnostics_keep_essence_and_drop_progress() {
        let raw = &format!(
            "{}error[E0425]: cannot find value `x` in this scope\n --> src/main.rs:2:14\n  |\n2 |     println!(\"{{x}}\");\n  |                ^ not found in this scope\n  = note: important note\n  = help: define `x` first\nwarning: unused variable: `y`\n --> src/main.rs:3:9\nerror: could not compile `demo` due to previous error\n",
            "   Compiling demo v0.1.0\n".repeat(20)
        );
        let data = compressed_for(&["cargo", "check"], "", raw);
        assert!(data.applied);
        assert_eq!(data.detected_kind, "cargo-build");
        assert!(data.stderr.contains("error[E0425]"));
        assert!(data.stderr.contains("src/main.rs:2:14"));
        assert!(data.stderr.contains("important note"));
        assert!(data.stderr.contains("define `x` first"));
        assert!(!data.stderr.contains("Compiling demo"));
    }

    #[test]
    fn cargo_test_summarizes_passes_and_keeps_failures() {
        let raw = format!(
            "running 52 tests\n{}test tests::broken ... FAILED\n\nfailures:\n\n---- tests::broken stdout ----\nthread 'tests::broken' panicked at src/lib.rs:7:9:\nassertion `left == right` failed\n  left: 1\n right: 2\nstack backtrace:\n{}\ntest result: FAILED. 51 passed; 1 failed; 0 ignored; finished in 0.01s\n",
            "test tests::ok ... ok\n".repeat(51),
            "   0: frame\n".repeat(30)
        );
        let data = compressed_for(&["cargo", "test"], &raw, "");
        assert!(data.applied);
        assert_eq!(data.detected_kind, "cargo-test");
        assert!(data.stdout.contains("test summary: passed=51 failed=1"));
        assert!(data.stdout.contains("tests::broken"));
        assert!(data.stdout.contains("assertion `left == right` failed"));
        assert!(data.stdout.contains("omitted 18 backtrace frames"));
        assert!(!data.stdout.contains("tests::ok ... ok\ntest tests::ok"));
    }

    #[test]
    fn generic_test_compression_preserves_failures_only() {
        let raw = format!(
            "{}FAIL tests/widget.test.ts > renders button\nAssertionError: expected 'a' to equal 'b'\n    at tests/widget.test.ts:9:3\n{}\n",
            "PASS tests/ok.test.ts\n".repeat(40),
            "✓ skipped thing\n".repeat(5)
        );
        let data = compressed_for(&["vitest"], &raw, "");
        assert!(data.applied);
        assert_eq!(data.detected_kind, "test-runner");
        assert!(
            data.stdout
                .contains("test summary: passed=40 failed=1 skipped=5")
        );
        assert!(data.stdout.contains("renders button"));
        assert!(data.stdout.contains("AssertionError"));
        assert!(
            !data
                .stdout
                .contains("PASS tests/ok.test.ts\nPASS tests/ok.test.ts")
        );
    }

    #[test]
    fn panic_backtrace_is_bounded() {
        let raw = format!(
            "test panic_case ... FAILED\n\nfailures:\nthread 'panic_case' panicked at src/main.rs:10:5:\nboom happened\nstack backtrace:\n{}test result: FAILED. 0 passed; 1 failed; 0 ignored\n",
            "   1: repeated::frame\n".repeat(40)
        );
        let data = compressed_for(&["cargo", "test"], &raw, "");
        assert!(data.applied);
        assert!(data.stdout.contains("boom happened"));
        assert!(data.stdout.contains("src/main.rs:10:5"));
        assert!(data.stdout.contains("omitted 28 backtrace frames"));
        assert!(data.stdout.len() < raw.len());
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
}
