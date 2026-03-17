# Change Proposal: extend-notify-set-output-matches

## Problem/Context

`add-notify-set-command` establishes `notify set` as the metadata-only entry point for persisted completion notification settings, but it does not cover output-match notifications.

- This session clarified that notification behavior must expand beyond `job.finished` to support commands that react when stdout/stderr lines match a configured filter.
- The user wants output-match delivery to fire on every match, not once per job.
- The current runtime already streams stdout/stderr into per-job log files, so output matching should attach to that line-processing path rather than introducing a second log ingestion system.
- To stay consistent with `add-notify-set-command`, notification settings should remain persisted in `meta.json` and be mutable via `notify set` without causing immediate delivery side effects.
- `normalize-global-root-option` is already active, so this proposal must reuse shared root selection behavior instead of redefining CLI root syntax.

## Proposed Solution

Extend the persisted notification model and `notify set` CLI so jobs can be configured with both completion notifications and output-match notifications.

- Extend `meta.json.notification` to distinguish completion sinks from output-match sinks while preserving the metadata-first model introduced by `add-notify-set-command`.
- Add `notify set` options for output-match configuration: pattern, match mode, stream selection, and command/file sinks.
- Keep `notify set` metadata-only for all job states: it updates persisted settings but never replays prior output or immediately invokes sinks.
- Make the running supervisor consult the latest persisted output-match configuration for future lines and emit `job.output.matched` events whenever a newly observed stdout/stderr line matches.
- Deliver output-match events through the same sink contract used for completion notifications: command sinks receive event JSON on stdin, file sinks append NDJSON, and failures are recorded without altering the main job state.
- Keep `job.finished` support intact and backward compatible, including the `notify set --command` completion-focused behavior defined by `add-notify-set-command`.

## Acceptance Criteria

- `notify set` can persist output-match settings for an existing job without executing any sink immediately.
- Output-match configuration can target `stdout`, `stderr`, or either stream and supports at least `contains` and `regex` matching modes.
- Once output-match settings are saved, newly observed matching lines trigger `job.output.matched` delivery for every match.
- If `notify set` updates a running job after it has already produced some output, only future lines are eligible for matching; prior output is not replayed.
- Command sinks for output matches execute through the configured shell wrapper and receive event JSON on stdin with event metadata in environment variables.
- File sinks for output matches append one JSON event per match as NDJSON and create parent directories when needed.
- Sink failures do not change the job lifecycle state and are captured in persisted notification records.
- Existing completion notification behavior and `add-notify-set-command` semantics remain valid.

## Out of Scope

- Adding `notify clear`, `notify show`, `notify replay`, or multi-rule output matching.
- Retrospective scanning of historical logs when output-match settings are added after job start.
- Redefining global `--root` syntax.
