//! Implementation of the `tag` sub-command and shared tag utilities.
//!
//! Tag format rules:
//! - A stored tag is a non-empty dot-separated sequence of segments.
//! - Each segment consists of alphanumeric characters and hyphens only.
//! - Tags may NOT end with `.*` (that syntax is reserved for list filter patterns).
//! - A list filter pattern is either an exact stored tag, or a stored-tag prefix
//!   terminated with `.*` (e.g. `hoge.*`, `hoge.fuga.*`).

use anyhow::Result;
use std::collections::HashSet;
use std::fmt;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{Response, TagSetData};

/// Error type for invalid tag values or filter patterns.
#[derive(Debug)]
pub struct InvalidTag {
    pub value: String,
    pub reason: &'static str,
}

impl fmt::Display for InvalidTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid tag {:?}: {}", self.value, self.reason)
    }
}

impl std::error::Error for InvalidTag {}

/// Validate a stored tag value.
///
/// A valid stored tag is a non-empty dot-separated sequence of segments where
/// each segment contains only alphanumeric characters and hyphens.
/// The `.*` suffix is not allowed in stored tags.
pub fn validate_stored_tag(tag: &str) -> Result<(), InvalidTag> {
    if tag.is_empty() {
        return Err(InvalidTag {
            value: tag.to_string(),
            reason: "tag must not be empty",
        });
    }
    if tag.ends_with(".*") {
        return Err(InvalidTag {
            value: tag.to_string(),
            reason: "stored tag may not end with '.*' (use exact tag names for run/tag set)",
        });
    }
    for segment in tag.split('.') {
        if segment.is_empty() {
            return Err(InvalidTag {
                value: tag.to_string(),
                reason: "tag segments must not be empty (no leading, trailing, or consecutive dots)",
            });
        }
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(InvalidTag {
                value: tag.to_string(),
                reason: "tag segments may only contain alphanumeric characters and hyphens",
            });
        }
    }
    Ok(())
}

/// Validate a list filter pattern.
///
/// Valid patterns are:
/// - An exact stored tag (validated by `validate_stored_tag`).
/// - A namespace prefix pattern ending in `.*` where the prefix before `.*`
///   is itself a valid stored tag.
pub fn validate_filter_pattern(pattern: &str) -> Result<(), InvalidTag> {
    if let Some(prefix) = pattern.strip_suffix(".*") {
        // Validate the prefix part as a stored tag.
        validate_stored_tag(prefix).map_err(|e| InvalidTag {
            value: pattern.to_string(),
            reason: e.reason,
        })
    } else {
        validate_stored_tag(pattern)
    }
}

/// Deduplicate tags, preserving first-seen order, and validate each one.
///
/// Returns an error if any tag is invalid.
pub fn dedup_tags(tags: Vec<String>) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for tag in tags {
        validate_stored_tag(&tag).map_err(anyhow::Error::from)?;
        if seen.insert(tag.clone()) {
            result.push(tag);
        }
    }
    Ok(result)
}

/// Check whether a job's tags satisfy all filter patterns (logical AND).
///
/// Returns true when every pattern matches at least one tag in `job_tags`.
pub fn matches_all_patterns(job_tags: &[String], patterns: &[String]) -> bool {
    patterns.iter().all(|pattern| {
        if let Some(prefix) = pattern.strip_suffix(".*") {
            // Namespace prefix match: at least one tag starts with "prefix."
            job_tags
                .iter()
                .any(|t| t == prefix || t.starts_with(&format!("{prefix}.")))
        } else {
            // Exact match.
            job_tags.iter().any(|t| t == pattern)
        }
    })
}

/// Options for the `tag set` sub-command.
pub struct TagOpts<'a> {
    pub root: Option<&'a str>,
    pub job_id: &'a str,
    pub tags: Vec<String>,
}

/// Execute `tag set`: replace tags on an existing job's meta.json atomically.
pub fn execute(opts: TagOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    // Validate and deduplicate the requested tags.
    let new_tags = dedup_tags(opts.tags)?;

    // Load the existing meta.json, update tags, and write back atomically.
    let mut meta = job_dir.read_meta()?;
    meta.tags = new_tags.clone();
    job_dir.write_meta_atomic(&meta)?;

    let response = Response::new(
        "tag_set",
        TagSetData {
            job_id: opts.job_id.to_string(),
            tags: new_tags,
        },
    );
    response.print();
    Ok(())
}
