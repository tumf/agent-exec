//! Embedded skill installation support for agent-exec.
//!
//! `install-skills` is intentionally narrow: it installs only the built-in
//! `agent-exec` skill into `.agents/skills/` or `.claude/skills/` and records
//! the result in `.skill-lock.json`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Embedded content of `skills/agent-exec/SKILL.md`.
const EMBEDDED_SKILL_MD: &[u8] = include_bytes!("../skills/agent-exec/SKILL.md");
const EMBEDDED_CLI_CONTRACT_MD: &[u8] =
    include_bytes!("../skills/agent-exec/references/cli-contract.md");
const EMBEDDED_COMPLETION_EVENTS_MD: &[u8] =
    include_bytes!("../skills/agent-exec/references/completion-events.md");
const EMBEDDED_OPENCLAW_MD: &[u8] = include_bytes!("../skills/agent-exec/references/openclaw.md");

/// Represents a single embedded file: relative path within the skill dir and content.
pub struct EmbeddedFile {
    pub relative_path: &'static str,
    pub content: &'static [u8],
}

/// All embedded files for the built-in `agent-exec` skill.
pub static EMBEDDED_AGENT_EXEC_FILES: &[EmbeddedFile] = &[
    EmbeddedFile {
        relative_path: "SKILL.md",
        content: EMBEDDED_SKILL_MD,
    },
    EmbeddedFile {
        relative_path: "references/cli-contract.md",
        content: EMBEDDED_CLI_CONTRACT_MD,
    },
    EmbeddedFile {
        relative_path: "references/completion-events.md",
        content: EMBEDDED_COMPLETION_EVENTS_MD,
    },
    EmbeddedFile {
        relative_path: "references/openclaw.md",
        content: EMBEDDED_OPENCLAW_MD,
    },
];

/// A single entry in `.skill-lock.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    /// Skill name (directory name under `.agents/skills/`).
    pub name: String,
    /// Source type string used when the skill was installed.
    pub source_type: String,
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
    pub fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(LockFile::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read lock file {}", path.display()))?;
        if let Ok(lock) = serde_json::from_str::<LockFile>(&raw) {
            return Ok(lock);
        }
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

    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs for {}", path.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize lock file")?;
        std::fs::write(path, json).with_context(|| format!("write lock file {}", path.display()))
    }

    pub fn upsert(&mut self, entry: LockEntry) {
        if let Some(existing) = self.skills.iter_mut().find(|e| e.name == entry.name) {
            *existing = entry;
        } else {
            self.skills.push(entry);
        }
    }
}

/// Result of installing the built-in skill.
#[derive(Debug, Clone)]
pub struct InstalledSkill {
    pub name: String,
    pub path: PathBuf,
    pub source_type: String,
}

/// Install the built-in `agent-exec` skill into `agents_dir/skills/agent-exec/`.
pub fn install_builtin(agents_dir: &Path) -> Result<InstalledSkill> {
    let name = "agent-exec";
    let dest = agents_dir.join("skills").join(name);
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
        source_type: "embedded".to_string(),
    })
}

/// Resolve the root directory for skill installation.
pub fn resolve_root_dir(global: bool, claude: bool) -> Result<PathBuf> {
    let root_name = if claude { ".claude" } else { ".agents" };
    if global {
        let home = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
            .home_dir()
            .to_path_buf();
        Ok(home.join(root_name))
    } else {
        let cwd = std::env::current_dir().context("get current directory")?;
        Ok(cwd.join(root_name))
    }
}

/// Get the timestamp in RFC 3339 format.
pub fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs;
    let sec = s % 60;
    let s = s / 60;
    let min = s % 60;
    let s = s / 60;
    let hour = s % 24;
    let days = s / 24;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
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
