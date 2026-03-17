# Design: extend-notify-set-output-matches

## Overview

This change extends the metadata-first notification model from `add-notify-set-command` so that `notify set` can configure output-match notifications in addition to completion notifications.

## Why this stays in `notify set`

- The active proposal `add-notify-set-command` already establishes `notify set` as the place to mutate persisted notification metadata after job creation.
- Output-match configuration has the same operational shape: it should be durable, mutable after job creation, and free of immediate side effects.
- Reusing `notify set` avoids creating two competing configuration entry points with different persistence semantics.

## Persisted notification model

The existing `meta.json.notification` object needs to grow from a flat completion-only shape into a structure that can hold two notification families:

- completion notification settings used for `job.finished`
- output-match notification settings used for `job.output.matched`

One acceptable shape is:

```json
{
  "notification": {
    "on_finish": {
      "command": "cat >/tmp/finish.json",
      "file": "/tmp/finish.ndjson"
    },
    "on_output_match": {
      "pattern": "ERROR",
      "match_type": "contains",
      "stream": "either",
      "command": "cat >/tmp/output.json",
      "file": "/tmp/output.ndjson"
    }
  }
}
```

Exact field names may vary, but the design requires:

- completion and output-match settings are independently addressable
- unspecified settings survive `notify set` updates
- the existing completion-focused `notify set --command` behavior remains representable

## Runtime model

There are two reload points for persisted notification metadata:

1. Right before terminal delivery for `job.finished`
2. During stdout/stderr line processing for future output lines

For output matching, the supervisor must evaluate only newly observed lines. It must not rescan historical logs when settings change. This keeps `notify set` metadata-only and avoids surprise replays.

## Matching semantics

- Matching is line-based.
- A line becomes eligible when a newline is observed or when EOF flushes a partial final line.
- `contains` performs substring matching on the lossy UTF-8 line written to logs.
- `regex` evaluates the same line using Rust regex syntax.
- `stdout`, `stderr`, or `either` decides which stream events are eligible.
- Every matching line emits a separate `job.output.matched` event.

## Delivery model

Output-match events reuse the existing sink contract:

- command sink: shell command string via configured shell wrapper, event JSON on stdin
- file sink: append one event JSON object as NDJSON
- environment metadata must expose the actual event type so consumers can distinguish `job.finished` from `job.output.matched`

Because output-match delivery can happen many times during a single job, delivery result persistence should use an append-only log such as `notification_events.ndjson` rather than overloading the single-object `completion_event.json` file.

## State and failure handling

- Delivery failures are best effort and must not alter the main job status.
- `completion_event.json` remains the terminal lifecycle record for `job.finished`.
- Output-match attempts are recorded separately with sink result metadata.
- `notify set` remains valid for any existing job state, but updates to terminal jobs only change metadata for inspection and future tooling; they do not replay prior output or send events immediately.

## Dependency on `add-notify-set-command`

This proposal is intentionally layered on top of `add-notify-set-command`.

- `add-notify-set-command` defines the metadata-only command shape and completion reload semantics.
- This change broadens that model to cover output-match settings and runtime matching.
- If both proposals are implemented together, this proposal should preserve the user-visible completion notification behavior introduced by the prerequisite change.
