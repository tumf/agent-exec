# Design: add-notify-set-command

## Overview

This proposal introduces a metadata-only `notify set` command. The key design choice is that `notify set` updates persisted notification configuration for any existing job without performing delivery itself.

## Why metadata-only

- It keeps `set` semantically narrow: update configuration, do not send events.
- It works for any present or future job state, including terminal jobs and possible future queued or scheduled jobs.
- It avoids ambiguous behavior such as auto-replay, duplicate delivery, or state-dependent side effects.

## Persisted model impact

The existing `meta.json.notification` structure is sufficient:

- `notify_command` is replaced with the new shell command string.
- `notify_file` is preserved when already present.
- If a job has no prior notification block, `notify set` creates `notification` with `notify_command` populated.

No change to `completion_event.json` schema is required.

## Completion delivery model

Today the supervisor receives notification settings from `run` launch options. To support post-creation updates, completion delivery needs one extra indirection:

1. `run` still persists initial notification metadata when present.
2. `notify set` mutates `meta.json.notification` later.
3. Right before terminal delivery, the supervisor reloads `meta.json`.
4. Delivery uses the latest persisted notification config.

This preserves current sink behavior while making post-creation updates observable at job finish time.

## Edge Cases

- Missing job: return `job_not_found` using the existing error envelope.
- Terminal job: update metadata successfully but do not execute the command.
- Existing `notify_file` only: `notify set` adds `notify_command` without removing the file sink.
- Existing `notify_command`: the new value replaces the previous command string.

## Interaction with global root normalization

This proposal intentionally avoids prescribing `--root` syntax. The active `normalize-global-root-option` change will decide how root selection is expressed at the CLI layer, and `notify set` should plug into that shared mechanism rather than redefining it here.
