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
    Pytest,
    Search,
    DockerLogs,
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
            Self::Json => "json",
            Self::Summary => "summary",
            Self::GitLog => "git-log",
            Self::CargoTest => "cargo-test",
            Self::Pytest => "pytest",
            Self::Search => "search",
            Self::DockerLogs => "docker-logs",
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
        "pytest" | "py.test" => Some(matched(
            DetectedKind::Pytest,
            "python",
            Some("test".to_string()),
        )),
        "rg" | "grep" => Some(matched(DetectedKind::Search, "search", None)),
        "ls" | "tree" | "find" => Some(matched(DetectedKind::List, "system", None)),
        "cat" | "head" | "tail" => Some(matched(DetectedKind::FileText, "system", None)),
        "jq" => Some(matched(DetectedKind::JsonStructure, "json", None)),
        "env" | "printenv" => Some(matched(DetectedKind::Env, "system", None)),
        "docker" if tokens.get(1).is_some_and(|token| token == "logs") => Some(matched(
            DetectedKind::DockerLogs,
            "containers",
            Some("logs".to_string()),
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
        assert_eq!(route.kind, DetectedKind::Pytest);
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
