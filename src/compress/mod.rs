use serde::{Deserialize, Serialize};

mod generic;
mod route;
mod util;

use route::{DetectedKind, route};
use util::{CompressionCandidate, guard_expansion};

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
        route(input.command, input.stdout, input.stderr).kind
    } else {
        mode_kind(input.mode)
    };

    let candidate = generic::compress_kind(kind, input.stdout, input.stderr);
    Some(guard_expansion(
        into_data(candidate, &input, kind),
        input.stdout,
        input.stderr,
    ))
}

fn into_data(
    candidate: CompressionCandidate,
    input: &CompressionInput<'_>,
    kind: DetectedKind,
) -> crate::schema::CompressionData {
    crate::schema::CompressionData {
        mode: input.mode.as_str().to_string(),
        applied: true,
        detected_kind: kind.as_str().to_string(),
        stdout_compressed_bytes: candidate.stdout.len() as u64,
        stderr_compressed_bytes: candidate.stderr.len() as u64,
        stdout_original_bytes: input.stdout_original_bytes,
        stderr_original_bytes: input.stderr_original_bytes,
        omitted: candidate.omitted,
        strategy: candidate.strategy,
        stdout: candidate.stdout,
        stderr: candidate.stderr,
    }
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
}
