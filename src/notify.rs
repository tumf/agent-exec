//! Implementation of the `notify` sub-commands.

use anyhow::Result;

use crate::jobstore::{JobDir, resolve_root};
use crate::schema::{
    NotificationConfig, NotifySetData, OutputMatchConfig, OutputMatchStream, OutputMatchType,
    Response,
};

/// Options for `notify set`.
pub struct NotifySetOpts<'a> {
    /// Job identifier.
    pub job_id: &'a str,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Shell command string to store as notify_command (completion notification).
    pub command: Option<String>,
    /// Pattern to match against output lines.
    pub output_pattern: Option<String>,
    /// Match type: "contains" or "regex".
    pub output_match_type: Option<String>,
    /// Stream selector: "stdout", "stderr", or "either".
    pub output_stream: Option<String>,
    /// Shell command string for output-match command sink.
    pub output_command: Option<String>,
    /// File path for output-match NDJSON file sink.
    pub output_file: Option<String>,
}

/// Execute `notify set`: update persisted notification configuration for an existing job.
///
/// This is a metadata-only operation: it rewrites meta.json.notification and
/// preserves unspecified fields. It does not execute any sink or trigger delivery.
pub fn set(opts: NotifySetOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let mut meta = job_dir.read_meta()?;

    // Preserve existing completion notification fields.
    let existing_notify_command = meta
        .notification
        .as_ref()
        .and_then(|n| n.notify_command.clone());
    let existing_notify_file = meta
        .notification
        .as_ref()
        .and_then(|n| n.notify_file.clone());
    let existing_on_output_match = meta
        .notification
        .as_ref()
        .and_then(|n| n.on_output_match.clone());

    // Update completion notify_command if provided.
    let new_notify_command = opts.command.or(existing_notify_command);

    // Build updated output-match config.
    let new_on_output_match = build_output_match_config(
        opts.output_pattern,
        opts.output_match_type,
        opts.output_stream,
        opts.output_command,
        opts.output_file,
        existing_on_output_match,
    );

    // Only write notification block if something is configured.
    let has_anything = new_notify_command.is_some()
        || existing_notify_file.is_some()
        || new_on_output_match.is_some();

    meta.notification = if has_anything {
        Some(NotificationConfig {
            notify_command: new_notify_command,
            notify_file: existing_notify_file,
            on_output_match: new_on_output_match,
        })
    } else {
        None
    };

    job_dir.write_meta_atomic(&meta)?;

    let notification = meta.notification.unwrap_or(NotificationConfig {
        notify_command: None,
        notify_file: None,
        on_output_match: None,
    });
    let response = Response::new(
        "notify.set",
        NotifySetData {
            job_id: opts.job_id.to_string(),
            notification,
        },
    );
    response.print();
    Ok(())
}

/// Build an updated `OutputMatchConfig` by merging provided options with existing config.
///
/// - If `output_pattern` is provided, a new config is created from scratch using
///   provided values (with defaults for unspecified fields).
/// - If no new pattern is provided but other output-match fields are provided,
///   they overlay the existing config.
/// - If nothing is provided and there's no existing config, returns `None`.
fn build_output_match_config(
    output_pattern: Option<String>,
    output_match_type: Option<String>,
    output_stream: Option<String>,
    output_command: Option<String>,
    output_file: Option<String>,
    existing: Option<OutputMatchConfig>,
) -> Option<OutputMatchConfig> {
    let has_new_input = output_pattern.is_some()
        || output_match_type.is_some()
        || output_stream.is_some()
        || output_command.is_some()
        || output_file.is_some();

    if !has_new_input {
        return existing;
    }

    // Start from existing config or defaults.
    let base = existing.unwrap_or_else(|| OutputMatchConfig {
        pattern: String::new(),
        match_type: OutputMatchType::default(),
        stream: OutputMatchStream::default(),
        command: None,
        file: None,
    });

    let pattern = output_pattern.unwrap_or(base.pattern);

    let match_type = match output_match_type.as_deref() {
        Some("regex") => OutputMatchType::Regex,
        Some("contains") => OutputMatchType::Contains,
        _ => base.match_type,
    };

    let stream = match output_stream.as_deref() {
        Some("stdout") => OutputMatchStream::Stdout,
        Some("stderr") => OutputMatchStream::Stderr,
        Some("either") => OutputMatchStream::Either,
        _ => base.stream,
    };

    // For command/file sinks: if provided, replace; otherwise preserve existing.
    let command = output_command.or(base.command);
    let file = output_file.or(base.file);

    // Only produce a config if there's a non-empty pattern.
    if pattern.is_empty() {
        return None;
    }

    Some(OutputMatchConfig {
        pattern,
        match_type,
        stream,
        command,
        file,
    })
}
