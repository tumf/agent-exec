//! Skill installation support for agent-exec.
//!
//! Provides:
//! - `Source` enum for parsing skill source specifications
//! - Embedded skill data for the built-in `agent-exec` skill
//! - Lock file reading/writing (`.agents/.skill-lock.json`)
//! - Skill expansion (copy to `.agents/skills/<name>/`)

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Error returned when an unrecognised source scheme is provided.
#[derive(Debug)]
pub struct UnknownSourceScheme(pub String);

impl std::fmt::Display for UnknownSourceScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown_source_scheme: {:?} (supported: self, local:<path>)",
            self.0
        )
    }
}

impl std::error::Error for UnknownSourceScheme {}

// ---------------------------------------------------------------------------
// Embedded skill files
// ---------------------------------------------------------------------------

/// Embedded content of `skills/agent-exec/SKILL.md`.
const EMBEDDED_SKILL_MD: &[u8] = include_bytes!("../skills/agent-exec/SKILL.md");

/// Represents a single embedded file: relative path within the skill dir and content.
pub struct EmbeddedFile {
    pub relative_path: &'static str,
    pub content: &'static [u8],
}

/// All embedded files for the built-in `agent-exec` skill.
pub static EMBEDDED_AGENT_EXEC_FILES: &[EmbeddedFile] = &[EmbeddedFile {
    relative_path: "SKILL.md",
    content: EMBEDDED_SKILL_MD,
}];

// ---------------------------------------------------------------------------
// Source
// ---------------------------------------------------------------------------

/// Skill installation source.
#[derive(Debug, Clone)]
pub enum Source {
    /// The built-in `agent-exec` skill (embedded in the binary).
    SelfEmbedded,
    /// A skill directory on the local filesystem.
    Local(PathBuf),
}

impl Source {
    /// Parse a source string such as `"self"` or `"local:/path/to/skill"`.
    ///
    /// Returns an error with `unknown_source_scheme` context when the scheme
    /// is not recognised.
    pub fn parse(s: &str) -> Result<Self> {
        if s == "self" {
            return Ok(Source::SelfEmbedded);
        }
        if let Some(path) = s.strip_prefix("local:") {
            return Ok(Source::Local(PathBuf::from(path)));
        }
        bail!(UnknownSourceScheme(s.to_string()));
    }
}

// ---------------------------------------------------------------------------
// Lock file
// ---------------------------------------------------------------------------

/// A single entry in `.skill-lock.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    /// Skill name (directory name under `.agents/skills/`).
    pub name: String,
    /// Source string used when the skill was installed.
    pub source: String,
    /// RFC 3339 timestamp of installation.
    pub installed_at: String,
    /// Absolute path to the installed skill directory.
    pub path: String,
}

/// Represents the `.agents/.skill-lock.json` file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LockFile {
    /// Ordered list of installed skills.
    pub skills: Vec<LockEntry>,
}

impl LockFile {
    /// Read the lock file from disk.  Returns an empty lock file if it does
    /// not exist.  Supports the legacy map format by ignoring unknown shapes
    /// gracefully.
    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(LockFile::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read lock file {}", path.display()))?;
        // Try the canonical array format first, then the legacy map format.
        if let Ok(lock) = serde_json::from_str::<LockFile>(&raw) {
            return Ok(lock);
        }
        // Legacy: the file might be a JSON object with a "skills" key that is
        // a map { name -> entry }.  Attempt to convert it.
        if let Ok(map) = serde_json::from_str::<serde_json::Value>(&raw)
            && let Some(obj) = map.get("skills").and_then(|v| v.as_object())
        {
            let mut skills = Vec::new();
            for (name, val) in obj {
                if let Ok(entry) = serde_json::from_value::<LockEntry>(val.clone()) {
                    let mut e = entry;
                    e.name = name.clone();
                    skills.push(e);
                }
            }
            return Ok(LockFile { skills });
        }
        Ok(LockFile::default())
    }

    /// Write the lock file to disk (canonical array format).
    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {}", path.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize lock file")?;
        std::fs::write(path, json).with_context(|| format!("write lock file {}", path.display()))
    }

    /// Update or insert an entry for `name`.
    pub fn upsert(&mut self, entry: LockEntry) {
        if let Some(existing) = self.skills.iter_mut().find(|e| e.name == entry.name) {
            *existing = entry;
        } else {
            self.skills.push(entry);
        }
    }
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

/// Result of installing a single skill.
#[derive(Debug, Clone)]
pub struct InstalledSkill {
    /// Skill name (directory name under `.agents/skills/`).
    pub name: String,
    /// Absolute path to the installed skill directory.
    pub path: PathBuf,
    /// Source string used.
    pub source_str: String,
}

/// Install a skill from `source` into `agents_dir/skills/<name>/`.
///
/// `agents_dir` is the `.agents/` directory (local or global).
///
/// Returns information about the installed skill.
pub fn install(source: &Source, agents_dir: &Path) -> Result<InstalledSkill> {
    let skills_dir = agents_dir.join("skills");
    match source {
        Source::SelfEmbedded => {
            let name = "agent-exec";
            let dest = skills_dir.join(name);
            std::fs::create_dir_all(&dest)
                .with_context(|| format!("create skill dir {}", dest.display()))?;
            for file in EMBEDDED_AGENT_EXEC_FILES {
                let file_dest = dest.join(file.relative_path);
                if let Some(parent) = file_dest.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("create parent dir {}", parent.display()))?;
                }
                std::fs::write(&file_dest, file.content)
                    .with_context(|| format!("write embedded file {}", file_dest.display()))?;
            }
            Ok(InstalledSkill {
                name: name.to_string(),
                path: dest,
                source_str: "self".to_string(),
            })
        }
        Source::Local(src_path) => {
            let name = src_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "cannot determine skill name from path: {}",
                        src_path.display()
                    )
                })?;
            let dest = skills_dir.join(name);
            copy_dir_local(src_path, &dest)?;
            Ok(InstalledSkill {
                name: name.to_string(),
                path: dest,
                source_str: format!("local:{}", src_path.display()),
            })
        }
    }
}

/// Copy a directory from `src` to `dst`.
///
/// On Unix, attempts to create a symlink first; falls back to recursive copy.
/// On Windows, always performs a recursive copy.
fn copy_dir_local(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        // Remove existing target if present.
        if dst.exists() || dst.symlink_metadata().is_ok() {
            if dst.is_symlink() || dst.is_file() {
                std::fs::remove_file(dst).ok();
            } else if dst.is_dir() {
                std::fs::remove_dir_all(dst).ok();
            }
        }
        // Canonicalize source for a stable symlink target.
        let abs_src = src
            .canonicalize()
            .with_context(|| format!("canonicalize local skill source path {}", src.display()))?;
        if symlink(&abs_src, dst).is_ok() {
            return Ok(());
        }
        // Symlink failed; fall back to copy.
        copy_dir_recursive(src, dst)
    }
    #[cfg(not(unix))]
    {
        copy_dir_recursive(src, dst)
    }
}

/// Recursively copy all files and directories from `src` to `dst`.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("create dir {}", dst.display()))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("read dir {}", src.display()))? {
        let entry = entry.with_context(|| format!("iterate dir {}", src.display()))?;
        let file_type = entry.file_type().context("get file type")?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).with_context(|| {
                format!("copy {} to {}", src_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

/// Resolve the `.agents/` directory.
///
/// If `global` is true, returns `~/.agents/`.
/// Otherwise returns `<cwd>/.agents/`.
pub fn resolve_agents_dir(global: bool) -> Result<PathBuf> {
    if global {
        let home = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
            .home_dir()
            .to_path_buf();
        Ok(home.join(".agents"))
    } else {
        let cwd = std::env::current_dir().context("get current directory")?;
        Ok(cwd.join(".agents"))
    }
}

/// Get the timestamp in RFC 3339 format.
pub fn now_rfc3339() -> String {
    // Use a simple implementation without external chrono dependency.
    // std::time provides SystemTime which we format manually.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as RFC 3339 UTC: YYYY-MM-DDTHH:MM:SSZ
    let s = secs;
    let sec = s % 60;
    let s = s / 60;
    let min = s % 60;
    let s = s / 60;
    let hour = s % 24;
    let days = s / 24;
    // Convert days since epoch to date.
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Gregorian calendar computation.
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &dim in &months {
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
