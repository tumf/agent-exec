//! Implementation of the `schema` subcommand.
//!
//! Reads `schema/agent-exec.schema.json` from the same directory as the
//! binary and returns it as a JSON response.

use anyhow::{Context, Result};

use crate::schema::{Response, SchemaData};

pub struct SchemaOpts;

/// Execute the `schema` subcommand.
///
/// Loads the bundled JSON Schema from a path relative to the binary and
/// prints a JSON envelope to stdout.
pub fn execute(_opts: SchemaOpts) -> Result<()> {
    // Locate schema relative to the binary directory so installs work
    // regardless of the working directory.
    let schema_path = schema_file_path()?;

    let raw = std::fs::read_to_string(&schema_path)
        .with_context(|| format!("failed to read schema file: {}", schema_path.display()))?;

    let schema: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| "schema file is not valid JSON")?;

    // Extract generated_at from the schema's `$comment` or use a static fallback.
    let generated_at = schema
        .get("$comment")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let data = SchemaData {
        schema_format: "json-schema-draft-07".to_string(),
        schema,
        generated_at,
    };

    Response::new("schema", data).print();
    Ok(())
}

/// Resolve the schema file path.
///
/// Looks for `schema/agent-exec.schema.json` relative to the binary.
fn schema_file_path() -> Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("cannot determine binary path")?;
    let bin_dir = exe.parent().context("binary has no parent directory")?;

    // During `cargo test` the binary is placed in target/debug/deps/;
    // try both the binary dir and its parent so that the schema file at
    // `<repo>/schema/agent-exec.schema.json` is found under `target/debug/`.
    for candidate in &[
        bin_dir.to_path_buf(),
        bin_dir.join(".."),
        bin_dir.join("../.."),
        bin_dir.join("../../.."),
    ] {
        let p = candidate.join("schema").join("agent-exec.schema.json");
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or(p));
        }
    }

    // Fallback: relative to CWD (useful in development).
    let cwd_path = std::path::Path::new("schema").join("agent-exec.schema.json");
    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    anyhow::bail!(
        "schema file not found; expected schema/agent-exec.schema.json relative to the binary or current directory"
    )
}
