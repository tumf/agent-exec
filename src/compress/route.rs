#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedKind {
    Errors,
    Tests,
    Logs,
    Git,
    GitStatus,
    GitLog,
    GitDiff,
    GitShow,
    GitPush,
    GitPull,
    GitBranch,
    GitStash,
    Json,
    Summary,
    CargoTest,
    TypeScript,
    JsLint,
    JsTest,
    JsPackages,
    PythonLint,
    PythonTypecheck,
    PythonTest,
    PythonPackages,
    GoDiagnostics,
    GoTest,
    Search,
    DockerTable,
    DockerLogs,
    KubernetesTable,
    KubernetesLogs,
    GitHubCli,
    GitLabCli,
    Aws,
    HttpTransfer,
    PsqlTable,
    JsonStructure,
    List,
    FileText,
    Env,
}

impl DetectedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Tests => "tests",
            Self::Logs => "logs",
            Self::Git => "git",
            Self::GitStatus => "git-status",
            Self::GitLog => "git-log",
            Self::GitDiff => "git-diff",
            Self::GitShow => "git-show",
            Self::GitPush => "git-push",
            Self::GitPull => "git-pull",
            Self::GitBranch => "git-branch",
            Self::GitStash => "git-stash",
            Self::Json => "json",
            Self::Summary => "summary",
            Self::CargoTest => "cargo-test",
            Self::TypeScript => "typescript",
            Self::JsLint => "js-lint",
            Self::JsTest => "js-test",
            Self::JsPackages => "js-packages",
            Self::PythonLint => "python-lint",
            Self::PythonTypecheck => "python-typecheck",
            Self::PythonTest => "python-test",
            Self::PythonPackages => "python-packages",
            Self::GoDiagnostics => "go-diagnostics",
            Self::GoTest => "go-test",
            Self::Search => "search",
            Self::DockerTable => "docker-table",
            Self::DockerLogs => "docker-logs",
            Self::KubernetesTable => "kubernetes-table",
            Self::KubernetesLogs => "kubernetes-logs",
            Self::GitHubCli => "github-cli",
            Self::GitLabCli => "gitlab-cli",
            Self::Aws => "aws",
            Self::HttpTransfer => "http-transfer",
            Self::PsqlTable => "psql-table",
            Self::JsonStructure => "json-structure",
            Self::List => "list",
            Self::FileText => "file-text",
            Self::Env => "env",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch {
    pub kind: DetectedKind,
    pub family: &'static str,
    pub subcommand: Option<String>,
}

pub fn route(command: &[String], stdout: &str, stderr: &str) -> RouteMatch {
    let tokens = command_tokens(command);
    route_command(&tokens).unwrap_or_else(|| route_output_shape(stdout, stderr))
}

fn route_output_shape(stdout: &str, stderr: &str) -> RouteMatch {
    for classifier in OUTPUT_SHAPE_CLASSIFIERS {
        if let Some(route) = classifier(stdout, stderr) {
            return route;
        }
    }
    matched(DetectedKind::Summary, "summary", None)
}

type OutputShapeClassifier = fn(&str, &str) -> Option<RouteMatch>;

const OUTPUT_SHAPE_CLASSIFIERS: &[OutputShapeClassifier] = &[
    classify_json_output,
    classify_repeated_logs,
    classify_test_output,
    classify_psql_table,
    classify_error_output,
];

fn classify_json_output(stdout: &str, stderr: &str) -> Option<RouteMatch> {
    (crate::compress::util::looks_like_json(stdout)
        || crate::compress::util::looks_like_json(stderr))
    .then(|| matched(DetectedKind::JsonStructure, "json", None))
}

fn classify_repeated_logs(stdout: &str, stderr: &str) -> Option<RouteMatch> {
    (crate::compress::util::has_repeated_adjacent_lines(stdout)
        || crate::compress::util::has_repeated_adjacent_lines(stderr)
        || crate::compress::util::has_repeated_normalized_log_lines(stdout)
        || crate::compress::util::has_repeated_normalized_log_lines(stderr))
    .then(|| matched(DetectedKind::Logs, "logs", None))
}

fn classify_test_output(stdout: &str, stderr: &str) -> Option<RouteMatch> {
    let combined = format!("{stdout}\n{stderr}");
    looks_like_test_output(&combined).then(|| matched(DetectedKind::Tests, "tests", None))
}

fn classify_psql_table(stdout: &str, stderr: &str) -> Option<RouteMatch> {
    (looks_like_psql_table(stdout) || looks_like_psql_table(stderr))
        .then(|| matched(DetectedKind::PsqlTable, "database", None))
}

fn classify_error_output(stdout: &str, stderr: &str) -> Option<RouteMatch> {
    let combined = format!("{stdout}\n{stderr}");
    looks_like_error_output(&combined).then(|| matched(DetectedKind::Errors, "errors", None))
}

fn route_command(tokens: &[String]) -> Option<RouteMatch> {
    let executable = tokens.first()?.as_str();
    match executable {
        "git" => {
            let subcommand = git_subcommand(tokens);
            let kind = match subcommand.as_deref() {
                Some("status") => DetectedKind::GitStatus,
                Some("log") => DetectedKind::GitLog,
                Some("diff") => DetectedKind::GitDiff,
                Some("show") => DetectedKind::GitShow,
                Some("push") => DetectedKind::GitPush,
                Some("pull") => DetectedKind::GitPull,
                Some("branch") => DetectedKind::GitBranch,
                Some("stash") => DetectedKind::GitStash,
                _ => DetectedKind::Git,
            };
            Some(matched(kind, "git", subcommand))
        }
        "cargo" if tokens.get(1).is_some_and(|token| token == "test") => Some(matched(
            DetectedKind::CargoTest,
            "rust",
            Some("test".to_string()),
        )),
        "tsc" => Some(matched(
            DetectedKind::TypeScript,
            "javascript",
            Some("tsc".to_string()),
        )),
        "eslint" | "biome" => Some(matched(
            DetectedKind::JsLint,
            "javascript",
            Some(executable.to_string()),
        )),
        "prettier" if tokens.iter().any(|token| token == "--check") => Some(matched(
            DetectedKind::JsLint,
            "javascript",
            Some("prettier-check".to_string()),
        )),
        "next" if tokens.get(1).is_some_and(|token| token == "build") => Some(matched(
            DetectedKind::TypeScript,
            "javascript",
            Some("next-build".to_string()),
        )),
        "npm" | "pnpm" | "yarn" => route_node_package_manager(executable, tokens),
        "pytest" | "py.test" => Some(matched(
            DetectedKind::PythonTest,
            "python",
            Some("test".to_string()),
        )),
        "ruff"
            if tokens
                .get(1)
                .is_some_and(|token| token == "check" || token == "format") =>
        {
            Some(matched(
                DetectedKind::PythonLint,
                "python",
                tokens.get(1).cloned(),
            ))
        }
        "mypy" => Some(matched(
            DetectedKind::PythonTypecheck,
            "python",
            Some("mypy".to_string()),
        )),
        "pip" => route_pip(tokens),
        "uv" if tokens.get(1).is_some_and(|token| token == "pip") => route_uv_pip(tokens),
        "go" => route_go(tokens),
        "golangci-lint" if tokens.get(1).is_some_and(|token| token == "run") => Some(matched(
            DetectedKind::GoDiagnostics,
            "go",
            Some("golangci-lint".to_string()),
        )),
        "rg" | "grep" => Some(matched(DetectedKind::Search, "search", None)),
        "ls" | "tree" | "find" => Some(matched(DetectedKind::List, "system", None)),
        "cat" | "head" | "tail" => Some(matched(DetectedKind::FileText, "system", None)),
        "jq" => Some(matched(DetectedKind::JsonStructure, "json", None)),
        "env" | "printenv" => Some(matched(DetectedKind::Env, "system", None)),
        "docker" => route_docker(tokens),
        "kubectl" => route_kubectl(tokens),
        "gh" => Some(matched(
            DetectedKind::GitHubCli,
            "github",
            cli_subcommand(tokens),
        )),
        "glab" => Some(matched(
            DetectedKind::GitLabCli,
            "gitlab",
            cli_subcommand(tokens),
        )),
        "aws" => Some(matched(DetectedKind::Aws, "aws", cli_subcommand(tokens))),
        "curl" | "wget" => Some(matched(
            DetectedKind::HttpTransfer,
            "http-transfer",
            Some(executable.to_string()),
        )),
        "psql" => Some(matched(
            DetectedKind::PsqlTable,
            "database",
            cli_subcommand(tokens),
        )),
        _ => None,
    }
}

fn route_docker(tokens: &[String]) -> Option<RouteMatch> {
    if tokens.get(1).is_some_and(|token| token == "logs") {
        return Some(matched(
            DetectedKind::DockerLogs,
            "containers",
            Some("logs".to_string()),
        ));
    }
    if tokens.get(1).is_some_and(|token| token == "compose") {
        let subcommand = tokens.get(2).cloned();
        let kind = if subcommand.as_deref() == Some("logs") {
            DetectedKind::DockerLogs
        } else {
            DetectedKind::DockerTable
        };
        return Some(matched(kind, "containers", subcommand));
    }
    Some(matched(
        DetectedKind::DockerTable,
        "containers",
        cli_subcommand(tokens),
    ))
}

fn route_kubectl(tokens: &[String]) -> Option<RouteMatch> {
    let subcommand = cli_subcommand(tokens);
    let kind = if subcommand.as_deref() == Some("logs") {
        DetectedKind::KubernetesLogs
    } else {
        DetectedKind::KubernetesTable
    };
    Some(matched(kind, "kubernetes", subcommand))
}

fn cli_subcommand(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .skip(1)
        .find(|token| !token.starts_with('-') && !token.contains('='))
        .cloned()
}

fn git_subcommand(tokens: &[String]) -> Option<String> {
    let mut skip_value = false;
    tokens.iter().skip(1).find_map(|token| {
        if skip_value {
            skip_value = false;
            return None;
        }
        if matches!(token.as_str(), "-C" | "-c" | "--git-dir" | "--work-tree") {
            skip_value = true;
            return None;
        }
        if token.starts_with('-') || token.contains('=') {
            return None;
        }
        Some(token.clone())
    })
}

fn route_node_package_manager(executable: &str, tokens: &[String]) -> Option<RouteMatch> {
    match tokens.get(1).map(String::as_str) {
        Some("test") => Some(matched(
            DetectedKind::JsTest,
            "javascript",
            Some("test".to_string()),
        )),
        Some("run") if tokens.get(2).is_some_and(|token| token == "test") => Some(matched(
            DetectedKind::JsTest,
            "javascript",
            Some("test".to_string()),
        )),
        Some("install" | "add" | "list" | "ls" | "outdated") => Some(matched(
            DetectedKind::JsPackages,
            "javascript",
            Some(format!("{executable}-packages")),
        )),
        _ => None,
    }
}

fn route_pip(tokens: &[String]) -> Option<RouteMatch> {
    match tokens.get(1).map(String::as_str) {
        Some("list" | "outdated" | "freeze") => Some(matched(
            DetectedKind::PythonPackages,
            "python",
            Some("pip-packages".to_string()),
        )),
        _ => None,
    }
}

fn route_uv_pip(tokens: &[String]) -> Option<RouteMatch> {
    match tokens.get(2).map(String::as_str) {
        Some("list" | "outdated" | "freeze") => Some(matched(
            DetectedKind::PythonPackages,
            "python",
            Some("uv-pip-packages".to_string()),
        )),
        _ => None,
    }
}

fn route_go(tokens: &[String]) -> Option<RouteMatch> {
    match tokens.get(1).map(String::as_str) {
        Some("test") => Some(matched(
            DetectedKind::GoTest,
            "go",
            Some("test".to_string()),
        )),
        Some("build" | "vet") => Some(matched(
            DetectedKind::GoDiagnostics,
            "go",
            tokens.get(1).cloned(),
        )),
        _ => None,
    }
}

fn command_tokens(command: &[String]) -> Vec<String> {
    if command.len() >= 3 && shell_executable(&command[0]) && command[1] == "-c" {
        return command[2]
            .split_whitespace()
            .map(normalize_token)
            .filter(|token| !token.is_empty())
            .collect();
    }
    command.iter().map(|token| normalize_token(token)).collect()
}

fn normalize_token(token: &str) -> String {
    token
        .trim_matches(|c: char| c == '\'' || c == '"')
        .rsplit('/')
        .next()
        .unwrap_or(token)
        .to_ascii_lowercase()
}

fn shell_executable(token: &str) -> bool {
    matches!(normalize_token(token).as_str(), "sh" | "bash" | "zsh")
}

fn matched(kind: DetectedKind, family: &'static str, subcommand: Option<String>) -> RouteMatch {
    RouteMatch {
        kind,
        family,
        subcommand,
    }
}

fn looks_like_test_output(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("test") && (lower.contains("failed") || lower.contains("passed"))
}

fn looks_like_error_output(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("error") || lower.contains("panic") || lower.contains("traceback")
}

fn looks_like_psql_table(text: &str) -> bool {
    let mut has_separator = false;
    let mut has_row_count = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains('|') && trimmed.contains("---") {
            has_separator = true;
        }
        if trimmed.starts_with('(') && trimmed.ends_with("rows)") {
            has_row_count = true;
        }
    }
    has_separator || has_row_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn classifies_git_log() {
        let route = route(&cmd(&["git", "log", "--oneline"]), "", "");
        assert_eq!(route.kind, DetectedKind::GitLog);
        assert_eq!(route.subcommand.as_deref(), Some("log"));
    }

    #[test]
    fn classifies_cargo_test() {
        let route = route(&cmd(&["cargo", "test"]), "", "");
        assert_eq!(route.kind, DetectedKind::CargoTest);
    }

    #[test]
    fn classifies_pytest() {
        let route = route(&cmd(&["pytest", "tests"]), "", "");
        assert_eq!(route.kind, DetectedKind::PythonTest);
    }

    #[test]
    fn classifies_javascript_tooling() {
        assert_eq!(
            route(&cmd(&["tsc", "--noEmit"]), "", "").kind,
            DetectedKind::TypeScript
        );
        assert_eq!(
            route(&cmd(&["eslint", "."]), "", "").kind,
            DetectedKind::JsLint
        );
        assert_eq!(
            route(&cmd(&["biome", "check", "."]), "", "").kind,
            DetectedKind::JsLint
        );
        assert_eq!(
            route(&cmd(&["next", "build"]), "", "").kind,
            DetectedKind::TypeScript
        );
        assert_eq!(
            route(&cmd(&["prettier", "--check", "."]), "", "").kind,
            DetectedKind::JsLint
        );
        assert_eq!(
            route(&cmd(&["npm", "test"]), "", "").kind,
            DetectedKind::JsTest
        );
        assert_eq!(
            route(&cmd(&["yarn", "run", "test"]), "", "").kind,
            DetectedKind::JsTest
        );
        assert_eq!(
            route(&cmd(&["npm", "install"]), "", "").kind,
            DetectedKind::JsPackages
        );
        assert_eq!(
            route(&cmd(&["pnpm", "list"]), "", "").kind,
            DetectedKind::JsPackages
        );
    }

    #[test]
    fn classifies_python_tooling() {
        assert_eq!(
            route(&cmd(&["ruff", "check", "."]), "", "").kind,
            DetectedKind::PythonLint
        );
        assert_eq!(
            route(&cmd(&["ruff", "format", "."]), "", "").kind,
            DetectedKind::PythonLint
        );
        assert_eq!(
            route(&cmd(&["mypy", "src"]), "", "").kind,
            DetectedKind::PythonTypecheck
        );
        assert_eq!(
            route(&cmd(&["pip", "list"]), "", "").kind,
            DetectedKind::PythonPackages
        );
        assert_eq!(
            route(&cmd(&["uv", "pip", "outdated"]), "", "").kind,
            DetectedKind::PythonPackages
        );
    }

    #[test]
    fn classifies_go_tooling() {
        assert_eq!(
            route(&cmd(&["go", "test", "./..."]), "", "").kind,
            DetectedKind::GoTest
        );
        assert_eq!(
            route(&cmd(&["go", "build", "./..."]), "", "").kind,
            DetectedKind::GoDiagnostics
        );
        assert_eq!(
            route(&cmd(&["go", "vet", "./..."]), "", "").kind,
            DetectedKind::GoDiagnostics
        );
        assert_eq!(
            route(&cmd(&["golangci-lint", "run"]), "", "").kind,
            DetectedKind::GoDiagnostics
        );
    }

    #[test]
    fn classifies_search() {
        let rg_route = route(&cmd(&["rg", "needle"]), "", "");
        assert_eq!(rg_route.kind, DetectedKind::Search);
        let grep_route = route(&cmd(&["grep", "-R", "needle", "."]), "", "");
        assert_eq!(grep_route.kind, DetectedKind::Search);
    }

    #[test]
    fn classifies_system_list_read_json_and_env_commands() {
        for args in [
            &["ls", "src"][..],
            &["tree", "src"][..],
            &["find", ".", "-type", "f"][..],
        ] {
            assert_eq!(route(&cmd(args), "", "").kind, DetectedKind::List);
        }
        for args in [&["cat", "Cargo.toml"][..], &["tail", "-n", "20", "log"][..]] {
            assert_eq!(route(&cmd(args), "", "").kind, DetectedKind::FileText);
        }
        assert_eq!(
            route(&cmd(&["jq", ".items"]), "", "").kind,
            DetectedKind::JsonStructure
        );
        assert_eq!(route(&cmd(&["env"]), "", "").kind, DetectedKind::Env);
    }

    #[test]
    fn classifies_docker_logs() {
        let route = route(&cmd(&["docker", "logs", "app"]), "", "");
        assert_eq!(route.kind, DetectedKind::DockerLogs);
    }

    #[test]
    fn classifies_container_cloud_and_http_commands() {
        assert_eq!(
            route(&cmd(&["docker", "ps"]), "", "").kind,
            DetectedKind::DockerTable
        );
        assert_eq!(
            route(&cmd(&["docker", "compose", "ps"]), "", "").kind,
            DetectedKind::DockerTable
        );
        assert_eq!(
            route(&cmd(&["kubectl", "get", "pods"]), "", "").kind,
            DetectedKind::KubernetesTable
        );
        assert_eq!(
            route(&cmd(&["kubectl", "logs", "pod/app"]), "", "").kind,
            DetectedKind::KubernetesLogs
        );
        assert_eq!(
            route(&cmd(&["gh", "pr", "list"]), "", "").kind,
            DetectedKind::GitHubCli
        );
        assert_eq!(
            route(&cmd(&["glab", "issue", "view"]), "", "").kind,
            DetectedKind::GitLabCli
        );
        assert_eq!(
            route(&cmd(&["aws", "sts", "get-caller-identity"]), "", "").kind,
            DetectedKind::Aws
        );
        assert_eq!(
            route(&cmd(&["curl", "-I", "https://example.com"]), "", "").kind,
            DetectedKind::HttpTransfer
        );
        assert_eq!(
            route(&cmd(&["wget", "https://example.com"]), "", "").kind,
            DetectedKind::HttpTransfer
        );
        assert_eq!(
            route(&cmd(&["psql", "-c", "select 1"]), "", "").kind,
            DetectedKind::PsqlTable
        );
    }

    #[test]
    fn classifies_json_output() {
        let route = route(&cmd(&["tool"]), "{\"ok\":true}", "");
        assert_eq!(route.kind, DetectedKind::JsonStructure);
    }

    #[test]
    fn output_shape_priority_keeps_json_before_repeated_logs() {
        let route = route(&cmd(&["tool"]), "{\"ok\":true}\n{\"ok\":true}\n", "");
        assert_eq!(route.kind, DetectedKind::JsonStructure);
    }

    #[test]
    fn output_shape_priority_keeps_repeated_logs_before_errors() {
        let route = route(&cmd(&["tool"]), "ERROR same\nERROR same\n", "");
        assert_eq!(route.kind, DetectedKind::Logs);
    }

    #[test]
    fn command_family_priority_beats_output_shape_fallback() {
        let route = route(&cmd(&["git", "status"]), "{\"ok\":true}\n", "");
        assert_eq!(route.kind, DetectedKind::GitStatus);
    }

    #[test]
    fn classifies_repeated_logs() {
        let route = route(&cmd(&["tool"]), "same\nsame\n", "");
        assert_eq!(route.kind, DetectedKind::Logs);
    }

    #[test]
    fn classifies_timestamp_normalized_repeated_error_logs_before_errors() {
        let stdout =
            "2026-01-01T00:00:00Z ERROR retry failed\n2026-01-01T00:00:01Z ERROR retry failed\n";
        let route = route(&cmd(&["tool"]), stdout, "");
        assert_eq!(route.kind, DetectedKind::Logs);
    }

    #[test]
    fn classifies_stderr_timestamp_normalized_repeated_error_logs_before_errors() {
        let stderr =
            "2026-01-01T00:00:00Z ERROR retry failed\n2026-01-01T00:00:01Z ERROR retry failed\n";
        let route = route(&cmd(&["tool"]), "", stderr);
        assert_eq!(route.kind, DetectedKind::Logs);
    }

    #[test]
    fn classifies_single_error_as_errors() {
        let route = route(&cmd(&["tool"]), "ERROR one-off failure\n", "");
        assert_eq!(route.kind, DetectedKind::Errors);
    }

    #[test]
    fn classifies_unknown_commands_as_summary() {
        let route = route(&cmd(&["tool"]), "hello", "");
        assert_eq!(route.kind, DetectedKind::Summary);
    }

    #[test]
    fn classifies_shell_wrapped_command() {
        let route = route(&cmd(&["sh", "-c", "git log --oneline"]), "", "");
        assert_eq!(route.kind, DetectedKind::GitLog);
    }
}
