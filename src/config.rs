//! Configuration loading for agent-exec.
//!
//! Reads `config.toml` from the XDG config directory with optional CLI overrides.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level config struct for `config.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct AgentExecConfig {
    #[serde(default)]
    pub shell: ShellConfig,
}

/// `[shell]` section of `config.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct ShellConfig {
    /// Shell wrapper argv for Unix-like platforms (e.g. `["sh", "-lc"]`).
    pub unix: Option<Vec<String>>,
    /// Shell wrapper argv for Windows (e.g. `["cmd", "/C"]`).
    pub windows: Option<Vec<String>>,
}

/// Discover the default XDG config file path.
///
/// Returns `$XDG_CONFIG_HOME/agent-exec/config.toml` if `XDG_CONFIG_HOME` is set,
/// otherwise returns `~/.config/agent-exec/config.toml`.
pub fn discover_config_path() -> Option<PathBuf> {
    use directories::BaseDirs;
    let base = BaseDirs::new()?;
    Some(base.config_dir().join("agent-exec").join("config.toml"))
}

/// Load and parse a config file from the given path.
///
/// Returns `Ok(None)` if the file does not exist.
/// Returns `Err` if the file exists but cannot be parsed.
pub fn load_config(path: &Path) -> Result<Option<AgentExecConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read config file {}", path.display()))?;
    let cfg: AgentExecConfig = toml::from_str(&raw)
        .with_context(|| format!("parse config file {}", path.display()))?;
    Ok(Some(cfg))
}

/// Return the built-in platform default shell wrapper argv.
pub fn default_shell_wrapper() -> Vec<String> {
    #[cfg(not(windows))]
    return vec!["sh".to_string(), "-lc".to_string()];
    #[cfg(windows)]
    return vec!["cmd".to_string(), "/C".to_string()];
}

/// Parse a CLI `--shell-wrapper` string (e.g. `"bash -lc"`) into an argv vec.
///
/// Splits on whitespace; returns an error if the result is empty.
pub fn parse_shell_wrapper_str(s: &str) -> Result<Vec<String>> {
    let argv: Vec<String> = s.split_whitespace().map(|p| p.to_string()).collect();
    if argv.is_empty() {
        anyhow::bail!("--shell-wrapper must not be empty");
    }
    Ok(argv)
}

/// Resolve the effective shell wrapper from CLI override, config file, and built-in defaults.
///
/// Resolution order:
/// 1. `cli_override` from `--shell-wrapper`
/// 2. Config file at `config_path_override` (from `--config`)
/// 3. Default XDG config file
/// 4. Built-in platform default
pub fn resolve_shell_wrapper(
    cli_override: Option<&str>,
    config_path_override: Option<&str>,
) -> Result<Vec<String>> {
    // 1. CLI override takes highest precedence.
    if let Some(s) = cli_override {
        return parse_shell_wrapper_str(s);
    }

    // 2 & 3. Try explicit config path, then default XDG path.
    let config_path: Option<PathBuf> = if let Some(p) = config_path_override {
        Some(PathBuf::from(p))
    } else {
        discover_config_path()
    };

    if let Some(ref path) = config_path {
        if let Some(cfg) = load_config(path)? {
            if let Some(w) = platform_wrapper_from_config(&cfg.shell) {
                if w.is_empty() {
                    anyhow::bail!(
                        "config file shell wrapper must not be empty (from {})",
                        path.display()
                    );
                }
                return Ok(w);
            }
        }
    }

    // 4. Built-in platform default.
    Ok(default_shell_wrapper())
}

/// Extract the active platform's wrapper from `ShellConfig`.
fn platform_wrapper_from_config(cfg: &ShellConfig) -> Option<Vec<String>> {
    #[cfg(not(windows))]
    return cfg.unix.clone();
    #[cfg(windows)]
    return cfg.windows.clone();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_wrapper_is_nonempty() {
        let w = default_shell_wrapper();
        assert!(!w.is_empty());
    }

    #[test]
    fn parse_shell_wrapper_str_splits_whitespace() {
        let w = parse_shell_wrapper_str("bash -lc").unwrap();
        assert_eq!(w, vec!["bash", "-lc"]);
    }

    #[test]
    fn parse_shell_wrapper_str_rejects_empty() {
        assert!(parse_shell_wrapper_str("").is_err());
        assert!(parse_shell_wrapper_str("   ").is_err());
    }

    #[test]
    fn resolve_cli_override_takes_precedence() {
        let w = resolve_shell_wrapper(Some("bash -lc"), None).unwrap();
        assert_eq!(w, vec!["bash", "-lc"]);
    }

    #[test]
    fn resolve_missing_config_returns_default() {
        // Point to a nonexistent config; should fall back to default.
        let w = resolve_shell_wrapper(None, Some("/nonexistent/config.toml")).unwrap();
        assert_eq!(w, default_shell_wrapper());
    }

    #[test]
    fn load_config_parses_unix_wrapper() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), r#"[shell]
unix = ["bash", "-lc"]
"#)
        .unwrap();
        let cfg = load_config(tmp.path()).unwrap().unwrap();
        assert_eq!(cfg.shell.unix, Some(vec!["bash".to_string(), "-lc".to_string()]));
    }

    #[test]
    fn resolve_config_file_override_is_used() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "[shell]\nunix = [\"bash\", \"-lc\"]\nwindows = [\"cmd\", \"/C\"]\n",
        )
        .unwrap();
        let w = resolve_shell_wrapper(None, Some(tmp.path().to_str().unwrap())).unwrap();
        // On non-Windows the unix key is used; on Windows the windows key.
        #[cfg(not(windows))]
        assert_eq!(w, vec!["bash", "-lc"]);
        #[cfg(windows)]
        assert_eq!(w, vec!["cmd", "/C"]);
    }
}
