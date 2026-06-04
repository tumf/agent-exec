#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedKind {
    Errors,
    Tests,
    Logs,
    Git,
    Json,
    Summary,
    GitLog,
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
    DockerLogs,
    JsonStructure,
}

impl DetectedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Tests => "tests",
            Self::Logs => "logs",
            Self::Git => "git",
            Self::Json => "json",
            Self::Summary => "summary",
            Self::GitLog => "git-log",
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
            Self::DockerLogs => "docker-logs",
            Self::JsonStructure => "json-structure",
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
    if let Some(route) = route_command(&tokens) {
        return route;
    }

    let combined = format!("{stdout}\n{stderr}");
    if crate::compress::util::looks_like_json(stdout)
        || crate::compress::util::looks_like_json(stderr)
    {
        return matched(DetectedKind::JsonStructure, "json", None);
    }
    if looks_like_test_output(&combined) {
        return matched(DetectedKind::Tests, "tests", None);
    }
    if crate::compress::util::has_repeated_adjacent_lines(stdout)
        || crate::compress::util::has_repeated_adjacent_lines(stderr)
    {
        return matched(DetectedKind::Logs, "logs", None);
    }
    if looks_like_error_output(&combined) {
        return matched(DetectedKind::Errors, "errors", None);
    }
    matched(DetectedKind::Summary, "summary", None)
}

fn route_command(tokens: &[String]) -> Option<RouteMatch> {
    let executable = tokens.first()?.as_str();
    match executable {
        "git" => {
            let subcommand = tokens.get(1).cloned();
            let kind = if subcommand.as_deref() == Some("log") {
                DetectedKind::GitLog
            } else {
                DetectedKind::Git
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
        "docker" if tokens.get(1).is_some_and(|token| token == "logs") => Some(matched(
            DetectedKind::DockerLogs,
            "containers",
            Some("logs".to_string()),
        )),
        _ => None,
    }
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
        let route = route(&cmd(&["rg", "needle"]), "", "");
        assert_eq!(route.kind, DetectedKind::Search);
    }

    #[test]
    fn classifies_docker_logs() {
        let route = route(&cmd(&["docker", "logs", "app"]), "", "");
        assert_eq!(route.kind, DetectedKind::DockerLogs);
    }

    #[test]
    fn classifies_json_output() {
        let route = route(&cmd(&["tool"]), "{\"ok\":true}", "");
        assert_eq!(route.kind, DetectedKind::JsonStructure);
    }

    #[test]
    fn classifies_repeated_logs() {
        let route = route(&cmd(&["tool"]), "same\nsame\n", "");
        assert_eq!(route.kind, DetectedKind::Logs);
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
