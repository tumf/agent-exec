//! Implementation of the `install-skills` subcommand.
//!
//! Installs the built-in `agent-exec` skill into `.agents/skills/` or
//! `.claude/skills/` and updates `.skill-lock.json`.

use anyhow::Result;

use crate::schema::{InstallSkillsData, InstalledSkillSummary, Response};
use crate::skills::{LockEntry, LockFile, install_builtin, now_rfc3339, resolve_root_dir};

/// Options for the `install-skills` subcommand.
pub struct InstallSkillsOpts {
    /// If true, install into the home directory; otherwise into cwd.
    pub global: bool,
    /// If true, use `.claude` root instead of `.agents`.
    pub claude: bool,
}

/// Execute the `install-skills` command.
///
/// Prints a single JSON response to stdout on success.
/// Returns an error on failure (caller maps to `ErrorResponse`).
pub fn execute(opts: InstallSkillsOpts) -> Result<()> {
    let root_dir = resolve_root_dir(opts.global, opts.claude)?;

    let installed = install_builtin(&root_dir)?;

    let lock_path = root_dir.join(".skill-lock.json");
    let mut lock = LockFile::read(&lock_path)?;
    let entry = LockEntry {
        name: installed.name.clone(),
        source_type: installed.source_type.clone(),
        installed_at: now_rfc3339(),
        path: installed.path.to_string_lossy().into_owned(),
    };
    lock.upsert(entry);
    lock.write(&lock_path)?;

    // Build and print the response.
    let data = InstallSkillsData {
        skills: vec![InstalledSkillSummary {
            name: installed.name,
            source_type: installed.source_type,
            path: installed.path.to_string_lossy().into_owned(),
        }],
        global: opts.global,
        lock_file_path: lock_path.to_string_lossy().into_owned(),
    };
    Response::new("install_skills", data).print();
    Ok(())
}
