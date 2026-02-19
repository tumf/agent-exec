//! Implementation of the `install-skills` subcommand.
//!
//! Installs one or more skills into `.agents/skills/` and updates
//! `.agents/.skill-lock.json`.

use anyhow::Result;

use crate::schema::{InstallSkillsData, InstalledSkillSummary, Response};
use crate::skills::{LockEntry, LockFile, Source, install, now_rfc3339, resolve_agents_dir};

/// Options for the `install-skills` subcommand.
pub struct InstallSkillsOpts<'a> {
    /// Source specification (e.g. `"self"` or `"local:/path/to/skill"`).
    pub source: &'a str,
    /// If true, install into `~/.agents/`; otherwise install into `./.agents/`.
    pub global: bool,
}

/// Execute the `install-skills` command.
///
/// Prints a single JSON response to stdout on success.
/// Returns an error on failure (caller maps to `ErrorResponse`).
pub fn execute(opts: InstallSkillsOpts<'_>) -> Result<()> {
    // Parse the source.
    let source = Source::parse(opts.source)?;

    // Resolve the `.agents/` directory.
    let agents_dir = resolve_agents_dir(opts.global)?;

    // Install the skill.
    let installed = install(&source, &agents_dir)?;

    // Read and update the lock file.
    let lock_path = agents_dir.join(".skill-lock.json");
    let mut lock = LockFile::read(&lock_path)?;
    let entry = LockEntry {
        name: installed.name.clone(),
        source: installed.source_str.clone(),
        installed_at: now_rfc3339(),
        path: installed.path.to_string_lossy().into_owned(),
    };
    lock.upsert(entry);
    lock.write(&lock_path)?;

    // Build and print the response.
    let data = InstallSkillsData {
        skills: vec![InstalledSkillSummary {
            name: installed.name,
            source: installed.source_str,
            path: installed.path.to_string_lossy().into_owned(),
        }],
        global: opts.global,
        lock_file_path: lock_path.to_string_lossy().into_owned(),
    };
    Response::new("install_skills", data).print();
    Ok(())
}
