//! Implementation of the `notify` sub-commands.

use anyhow::Result;

use crate::jobstore::{resolve_root, JobDir};
use crate::schema::{NotificationConfig, NotifySetData, Response};

/// Options for `notify set`.
pub struct NotifySetOpts<'a> {
    /// Job identifier.
    pub job_id: &'a str,
    /// Override for jobs root directory.
    pub root: Option<&'a str>,
    /// Shell command string to store as notify_command.
    pub command: String,
}

/// Execute `notify set`: update persisted notify_command for an existing job.
///
/// This is a metadata-only operation: it rewrites meta.json.notification.notify_command
/// and preserves notify_file. It does not execute the command or trigger any delivery.
pub fn set(opts: NotifySetOpts) -> Result<()> {
    let root = resolve_root(opts.root);
    let job_dir = JobDir::open(&root, opts.job_id)?;

    let mut meta = job_dir.read_meta()?;

    // Preserve existing notify_file, replace notify_command.
    let existing_file = meta
        .notification
        .as_ref()
        .and_then(|n| n.notify_file.clone());
    meta.notification = Some(NotificationConfig {
        notify_command: Some(opts.command),
        notify_file: existing_file,
    });

    job_dir.write_meta_atomic(&meta)?;

    let notification = meta.notification.expect("just set above");
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
