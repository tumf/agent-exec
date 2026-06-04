use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use serde_json::Value;

use crate::compress::route::DetectedKind;
use crate::compress::util::{CompressionCandidate, bounded_summary, summarize_text};

const MAX_GROUPS: usize = 24;
const MAX_EXAMPLES: usize = 3;
const MAX_PACKAGES: usize = 40;

pub fn compress_kind(
    kind: DetectedKind,
    raw_stdout: &str,
    raw_stderr: &str,
) -> Option<CompressionCandidate> {
    let strategy = strategy_for(kind)?;
    let stdout = compress_stream(kind, raw_stdout);
    let stderr = compress_stream(kind, raw_stderr);
    let omitted = stdout.len() < raw_stdout.len() || stderr.len() < raw_stderr.len();
    Some(CompressionCandidate {
        stdout,
        stderr,
        omitted,
        strategy: vec![strategy.to_string()],
    })
}

fn strategy_for(kind: DetectedKind) -> Option<&'static str> {
    match kind {
        DetectedKind::TypeScript | DetectedKind::JsLint => Some("js-diagnostic-grouping"),
        DetectedKind::JsTest => Some("js-test-summary"),
        DetectedKind::JsPackages => Some("js-package-summary"),
        DetectedKind::PythonLint | DetectedKind::PythonTypecheck => {
            Some("python-diagnostic-grouping")
        }
        DetectedKind::PythonTest => Some("python-test-summary"),
        DetectedKind::PythonPackages => Some("python-package-summary"),
        DetectedKind::GoDiagnostics => Some("go-diagnostic-grouping"),
        DetectedKind::GoTest => Some("go-test-summary"),
        _ => None,
    }
}

fn compress_stream(kind: DetectedKind, text: &str) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    match kind {
        DetectedKind::TypeScript | DetectedKind::JsLint => {
            diagnostics_summary(text, &js_diagnostics(text))
        }
        DetectedKind::PythonLint | DetectedKind::PythonTypecheck => {
            diagnostics_summary(text, &python_diagnostics(text))
        }
        DetectedKind::GoDiagnostics => diagnostics_summary(text, &go_diagnostics(text)),
        DetectedKind::JsTest | DetectedKind::PythonTest => test_summary(text),
        DetectedKind::GoTest => go_test_summary(text),
        DetectedKind::JsPackages | DetectedKind::PythonPackages => package_summary(text),
        _ => summarize_text(text),
    }
}

#[derive(Debug, Clone)]
struct Diagnostic {
    file: String,
    line: Option<u64>,
    column: Option<u64>,
    severity: String,
    code: Option<String>,
    message: String,
}

fn diagnostics_summary(raw: &str, diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return bounded_summary(raw, 20, 10, 2400);
    }

    let mut groups: BTreeMap<(String, String, String), Vec<&Diagnostic>> = BTreeMap::new();
    for diagnostic in diagnostics {
        let code = diagnostic.code.as_deref().unwrap_or("uncoded").to_string();
        groups
            .entry((diagnostic.file.clone(), diagnostic.severity.clone(), code))
            .or_default()
            .push(diagnostic);
    }

    let mut out = vec![format!(
        "diagnostics total={} groups={}",
        diagnostics.len(),
        groups.len()
    )];
    for ((file, severity, code), items) in groups.iter().take(MAX_GROUPS) {
        out.push(format!("{file} [{severity}] {code} count={}", items.len()));
        for item in items.iter().take(MAX_EXAMPLES) {
            let loc = match (item.line, item.column) {
                (Some(line), Some(column)) => format!("{line}:{column}"),
                (Some(line), None) => line.to_string(),
                _ => "?".to_string(),
            };
            out.push(format!("  {loc}: {}", compact_message(&item.message)));
        }
    }
    if groups.len() > MAX_GROUPS {
        out.push(format!(
            "... omitted {} diagnostic groups ...",
            groups.len() - MAX_GROUPS
        ));
    }
    out.join("\n")
}

fn js_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut out = json_diagnostics(text);
    if !out.is_empty() {
        return out;
    }

    let tsc = Regex::new(r"^(?P<file>[^\s:(][^:(]*\.(?:ts|tsx|js|jsx))\((?P<line>\d+),(?P<col>\d+)\):\s*(?P<sev>error|warning)\s+(?P<code>TS\d+):\s*(?P<msg>.+)$").unwrap();
    let eslint = Regex::new(r"^\s*(?P<line>\d+):(?P<col>\d+)\s+(?P<sev>error|warning)\s+(?P<msg>.+?)\s+(?P<code>[\w@/-]+)$").unwrap();
    let file_re = Regex::new(r"^[/\w_. -]+\.(?:ts|tsx|js|jsx)$").unwrap();
    let mut current_file = String::new();

    for line in text.lines() {
        if let Some(caps) = tsc.captures(line) {
            out.push(Diagnostic {
                file: caps["file"].to_string(),
                line: caps["line"].parse().ok(),
                column: caps["col"].parse().ok(),
                severity: caps["sev"].to_string(),
                code: Some(caps["code"].to_string()),
                message: caps["msg"].to_string(),
            });
            continue;
        }
        let trimmed = line.trim();
        if file_re.is_match(trimmed) {
            current_file = trimmed.to_string();
            continue;
        }
        if let Some(caps) = eslint.captures(line) {
            out.push(Diagnostic {
                file: if current_file.is_empty() {
                    "<unknown>".to_string()
                } else {
                    current_file.clone()
                },
                line: caps["line"].parse().ok(),
                column: caps["col"].parse().ok(),
                severity: caps["sev"].to_string(),
                code: Some(caps["code"].to_string()),
                message: caps["msg"].trim().to_string(),
            });
        }
    }
    out
}

fn python_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut out = json_diagnostics(text);
    if !out.is_empty() {
        return out;
    }
    let ruff = Regex::new(
        r"^(?P<file>[^:]+\.py):(?P<line>\d+):(?P<col>\d+):\s*(?P<code>[A-Z]+\d+)\s+(?P<msg>.+)$",
    )
    .unwrap();
    let mypy = Regex::new(r"^(?P<file>[^:]+\.py):(?P<line>\d+):\s*(?P<sev>error|note|warning):\s*(?P<msg>.+?)(?:\s+\[(?P<code>[\w-]+)\])?$" ).unwrap();
    for line in text.lines() {
        if let Some(caps) = ruff.captures(line) {
            out.push(Diagnostic {
                file: caps["file"].to_string(),
                line: caps["line"].parse().ok(),
                column: caps["col"].parse().ok(),
                severity: "error".to_string(),
                code: Some(caps["code"].to_string()),
                message: caps["msg"].to_string(),
            });
        } else if let Some(caps) = mypy.captures(line) {
            out.push(Diagnostic {
                file: caps["file"].to_string(),
                line: caps["line"].parse().ok(),
                column: None,
                severity: caps["sev"].to_string(),
                code: caps.name("code").map(|m| m.as_str().to_string()),
                message: caps["msg"].to_string(),
            });
        }
    }
    out
}

fn go_diagnostics(text: &str) -> Vec<Diagnostic> {
    let mut out = json_diagnostics(text);
    if !out.is_empty() {
        return out;
    }
    let re = Regex::new(r"^(?P<file>[^:\s]+\.go):(?P<line>\d+):(?:(?P<col>\d+):)?\s*(?P<msg>.+?)(?:\s+\((?P<code>[\w.-]+)\))?$" ).unwrap();
    for line in text.lines() {
        if let Some(caps) = re.captures(line.trim()) {
            out.push(Diagnostic {
                file: caps["file"].to_string(),
                line: caps["line"].parse().ok(),
                column: caps.name("col").and_then(|m| m.as_str().parse().ok()),
                severity: "error".to_string(),
                code: caps.name("code").map(|m| m.as_str().to_string()),
                message: caps["msg"].to_string(),
            });
        }
    }
    out
}

fn json_diagnostics(text: &str) -> Vec<Diagnostic> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_json_diagnostics(&value, &mut out);
    out
}

fn collect_json_diagnostics(value: &Value, out: &mut Vec<Diagnostic>) {
    match value {
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_json_diagnostics(item, out)),
        Value::Object(map) => {
            if let Some(file) = string_field(map, &["filePath", "filename", "file", "path"]) {
                if let Some(messages) = map.get("messages").and_then(Value::as_array) {
                    for message in messages {
                        if let Some(diag) = diagnostic_from_json_obj(message, Some(file.clone())) {
                            out.push(diag);
                        }
                    }
                } else if let Some(diag) = diagnostic_from_json_obj(value, Some(file)) {
                    out.push(diag);
                }
            }
            for child in map.values() {
                if matches!(child, Value::Array(_) | Value::Object(_)) {
                    collect_json_diagnostics(child, out);
                }
            }
        }
        _ => {}
    }
}

fn diagnostic_from_json_obj(value: &Value, default_file: Option<String>) -> Option<Diagnostic> {
    let map = value.as_object()?;
    let file = string_field(map, &["filePath", "filename", "file", "path"]).or(default_file)?;
    let message = string_field(map, &["message", "text", "detail", "reason"])?;
    let code = string_field(map, &["ruleId", "rule", "code", "check_name"]);
    let severity = string_field(map, &["severity", "level", "type"]).unwrap_or_else(|| {
        match map.get("severity").and_then(Value::as_i64) {
            Some(n) if n >= 2 => "error".to_string(),
            Some(_) => "warning".to_string(),
            None => "error".to_string(),
        }
    });
    Some(Diagnostic {
        file,
        line: numeric_field(map, &["line", "row"]),
        column: numeric_field(map, &["column", "col", "character"]),
        severity,
        code,
        message,
    })
}

fn string_field(map: &serde_json::Map<String, Value>, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        map.get(*name)
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}

fn numeric_field(map: &serde_json::Map<String, Value>, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| map.get(*name).and_then(Value::as_u64))
}

fn test_summary(text: &str) -> String {
    if let Some(summary) = json_test_summary(text) {
        return summary;
    }
    let mut failures = Vec::new();
    let mut summary = Vec::new();
    let mut pass_count = 0usize;
    let mut in_failure = false;
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("failed")
            || lower.contains("failures")
            || lower.contains("traceback")
            || lower.contains("assert")
            || lower.contains("error")
        {
            in_failure = true;
        }
        if lower.contains(" passed")
            || lower.contains(" failed")
            || lower.contains(" skipped")
            || lower.contains("test result")
            || lower.contains("tests run")
        {
            summary.push(line.to_string());
        } else if lower.contains(" passed") || lower.ends_with(" ok") {
            pass_count += 1;
        }
        if in_failure && failures.len() < 80 {
            failures.push(line.to_string());
        }
        if in_failure && line.trim().is_empty() {
            in_failure = false;
        }
    }
    let mut out = Vec::new();
    if pass_count > 0 {
        out.push(format!("passing test lines summarized={pass_count}"));
    }
    out.extend(summary.into_iter().take(20));
    out.extend(failures);
    if out.is_empty() {
        bounded_summary(text, 20, 10, 2400)
    } else {
        out.join("\n")
    }
}

fn go_test_summary(text: &str) -> String {
    if text.lines().any(|line| line.trim_start().starts_with('{')) {
        let ndjson = go_test_ndjson_summary(text);
        if !ndjson.is_empty() {
            return ndjson;
        }
    }
    let mut packages = BTreeMap::<String, String>::new();
    let mut failures = Vec::new();
    let mut pass_count = 0usize;
    for line in text.lines() {
        if line.starts_with("ok  ") || line.starts_with("?   ") {
            pass_count += 1;
        } else if line.starts_with("FAIL")
            || line.contains("--- FAIL:")
            || line.contains("panic:")
            || line.contains("Error Trace:")
        {
            failures.push(line.to_string());
            if line.starts_with("FAIL\t") || line.starts_with("FAIL ") {
                let pkg = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("<unknown>")
                    .to_string();
                packages.insert(pkg, "failed".to_string());
            }
        }
    }
    let mut out = vec![format!(
        "go test packages_passed_summarized={pass_count} packages_failed={}",
        packages.len()
    )];
    out.extend(
        packages
            .into_iter()
            .map(|(pkg, status)| format!("package {pkg}: {status}")),
    );
    out.extend(failures.into_iter().take(80));
    out.join("\n")
}

fn go_test_ndjson_summary(text: &str) -> String {
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut packages = BTreeSet::new();
    let mut failures = Vec::new();
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let action = value.get("Action").and_then(Value::as_str).unwrap_or("");
        let package = value
            .get("Package")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        match action {
            "pass" => pass += 1,
            "fail" => {
                fail += 1;
                packages.insert(package.to_string());
                if let Some(test) = value.get("Test").and_then(Value::as_str) {
                    failures.push(format!("FAIL {package}::{test}"));
                } else {
                    failures.push(format!("FAIL {package}"));
                }
            }
            "output" => {
                let output = value.get("Output").and_then(Value::as_str).unwrap_or("");
                if output.contains("FAIL") || output.contains("panic") || output.contains("Error") {
                    failures.push(format!("{package}: {}", output.trim()));
                }
            }
            _ => {}
        }
    }
    if pass == 0 && fail == 0 && failures.is_empty() {
        return String::new();
    }
    let mut out = vec![format!(
        "go test ndjson pass_events={pass} fail_events={fail} failed_packages={}",
        packages.len()
    )];
    out.extend(failures.into_iter().take(80));
    out.join("\n")
}

fn json_test_summary(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    let mut failed = Vec::new();
    let mut passed = 0usize;
    collect_json_tests(&value, &mut passed, &mut failed);
    if passed == 0 && failed.is_empty() {
        return None;
    }
    let mut out = vec![format!(
        "test json passed_summarized={passed} failed={}",
        failed.len()
    )];
    out.extend(failed.into_iter().take(80));
    Some(out.join("\n"))
}

fn collect_json_tests(value: &Value, passed: &mut usize, failed: &mut Vec<String>) {
    match value {
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_json_tests(item, passed, failed)),
        Value::Object(map) => {
            let name = string_field(map, &["name", "title", "fullName", "test"])
                .unwrap_or_else(|| "<unnamed>".to_string());
            if let Some(status) = string_field(map, &["status", "outcome", "state"]) {
                let lower = status.to_ascii_lowercase();
                if lower.contains("pass") {
                    *passed += 1;
                } else if lower.contains("fail") || lower.contains("error") {
                    failed.push(format!("FAILED {name}: {status}"));
                }
            }
            for child in map.values() {
                collect_json_tests(child, passed, failed);
            }
        }
        _ => {}
    }
}

fn package_summary(text: &str) -> String {
    if let Some(summary) = json_package_summary(text) {
        return summary;
    }
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Package")
            || trimmed.starts_with("Name")
            || trimmed.starts_with("---")
        {
            continue;
        }
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() >= 2 {
            rows.push(format!("{} {}", cols[0], cols[1]));
        }
    }
    let total = rows.len();
    let mut out = vec![format!(
        "packages total={total} shown={}",
        total.min(MAX_PACKAGES)
    )];
    out.extend(rows.into_iter().take(MAX_PACKAGES));
    if total > MAX_PACKAGES {
        out.push(format!("... omitted {} packages ...", total - MAX_PACKAGES));
    }
    out.join("\n")
}

fn json_package_summary(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text.trim()).ok()?;
    let items = value.as_array()?;
    let mut rows = Vec::new();
    for item in items {
        let Some(map) = item.as_object() else {
            continue;
        };
        let name = string_field(map, &["name", "Name", "package", "Package"])?;
        let version = string_field(map, &["version", "Version", "latest_version", "Latest"])
            .unwrap_or_else(|| "?".to_string());
        rows.push(format!("{name} {version}"));
    }
    let total = rows.len();
    let mut out = vec![format!(
        "packages total={total} shown={}",
        total.min(MAX_PACKAGES)
    )];
    out.extend(rows.into_iter().take(MAX_PACKAGES));
    Some(out.join("\n"))
}

fn compact_message(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.len() > 180 {
        format!("{}...", &trimmed[..180])
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_tsc_and_eslint_diagnostics() {
        let raw = "src/a.ts(1,2): error TS2322: Type 'string' is not assignable\nsrc/a.ts(3,4): error TS2322: Type 'number' is not assignable\n\n/abs/src/b.ts\n  5:7  warning  Unexpected any  @typescript-eslint/no-explicit-any\n";
        let out = compress_stream(DetectedKind::TypeScript, raw);
        assert!(out.contains("src/a.ts [error] TS2322 count=2"));
        assert!(out.contains("/abs/src/b.ts [warning] @typescript-eslint/no-explicit-any"));
    }

    #[test]
    fn groups_python_and_pip_outputs() {
        let raw = "pkg/a.py:10:5: F401 unused import\npkg/b.py:8: error: Incompatible return value [return-value]\n";
        let out = compress_stream(DetectedKind::PythonLint, raw);
        assert!(out.contains("F401"));
        assert!(out.contains("return-value"));

        let packages = (0..50)
            .map(|n| format!("pkg{n} 1.{n}.0"))
            .collect::<Vec<_>>()
            .join("\n");
        let summary = compress_stream(DetectedKind::PythonPackages, &packages);
        assert!(summary.contains("packages total=50 shown=40"));
        assert!(summary.contains("omitted 10 packages"));
    }

    #[test]
    fn summarizes_test_failures_and_go_ndjson() {
        let raw = "test ok_1 PASSED\ntest ok_2 PASSED\nFAIL tests/test_app.py::test_bad - AssertionError: boom\n================ 2 passed, 1 failed ================\n";
        let out = compress_stream(DetectedKind::PythonTest, raw);
        assert!(out.contains("test_bad"));
        assert!(out.contains("1 failed"));

        let ndjson = r#"{"Action":"pass","Package":"example.com/a","Test":"TestOk"}
{"Action":"fail","Package":"example.com/a","Test":"TestBad"}
{"Action":"output","Package":"example.com/a","Output":"--- FAIL: TestBad (0.00s)\n"}"#;
        let go = compress_stream(DetectedKind::GoTest, ndjson);
        assert!(go.contains("go test ndjson"));
        assert!(go.contains("TestBad"));
    }

    #[test]
    fn groups_go_lint_text() {
        let raw = "cmd/app/main.go:12:5: printf: fmt.Println arg list ends with redundant newline (govet)\n";
        let out = compress_stream(DetectedKind::GoDiagnostics, raw);
        assert!(out.contains("cmd/app/main.go"));
        assert!(out.contains("govet"));
    }
}
