# Change Proposal: add-job-tags

## Problem/Context

`agent-exec` can already filter `list` results by cwd and state, but jobs cannot carry stable user-defined tags for grouping related work across commands and sessions, and there is no CLI to assign tags to an already-created job.

- The current session established that tags must be repeatable on `run`, e.g. `agent-exec run --tag aaa --tag bbb -- <cmd>`.
- The requested list filtering model is Docker-label-like namespace matching, including exact matches such as `hoge.fuga.geho` and prefix matches such as `hoge.fuga.*`.
- Existing behavior already applies cwd filtering by default in `list`, so tag filtering must compose with that behavior rather than replacing it.
- The repository persists per-job metadata in `meta.json` and verifies list behavior through `tests/integration.rs`, so the proposal should keep the feature centered on persisted metadata plus list-time filtering.
- The repo already has a `notify set` proposal pattern for mutating persisted job metadata after creation, so a tag-management subcommand should follow the same metadata-first style.

## Proposed Solution

Add repeatable job tags to `run`, allow replacing tags on existing jobs via a dedicated subcommand, persist them in `meta.json`, surface them in JSON responses, and add repeatable tag filters to `list`.

- Add repeatable `--tag <TAG>` flags to `agent-exec run`.
- Add `agent-exec tag set <JOB_ID> --tag <TAG>...` to replace the persisted tags for an existing job.
- Persist deduplicated tags in `meta.json.tags` while preserving first-seen order.
- Return `tags` in the `run` response, `tag set` response, and in each `list.jobs[]` item so callers can inspect the assigned metadata.
- Add repeatable `--tag <PATTERN>` filters to `agent-exec list`.
- Support two filter forms only: exact match (`aaa`, `hoge.fuga.geho`) and namespace prefix match ending in `.*` (`hoge.*`, `hoge.fuga.*`).
- Combine repeated list tag filters with logical AND, while still combining with existing cwd/state filters.
- Make `tag set` metadata-only: it updates `meta.json.tags` atomically for any existing job state and does not change process execution.

This remains a single proposal because tag capture at creation time, post-creation tag replacement, persistence, response shape, and list filtering are tightly coupled parts of one user-visible tagging workflow.

## Acceptance Criteria

- `agent-exec run --tag aaa --tag bbb -- <cmd>` succeeds and persists `tags: ["aaa", "bbb"]` in `meta.json`.
- Repeated `--tag` values on `run` are deduplicated without reordering the first occurrence.
- `agent-exec tag set <job_id> --tag aaa --tag bbb` succeeds for any existing job and replaces `meta.json.tags` with `["aaa", "bbb"]`.
- `tag set` does not modify unrelated persisted metadata and does not restart or otherwise affect the job process.
- `agent-exec list --tag aaa` returns only jobs whose persisted tags contain `aaa`.
- `agent-exec list --tag hoge.fuga.*` returns jobs having at least one tag in that namespace prefix.
- `agent-exec list --tag aaa --tag bbb` returns only jobs satisfying both tag filters.
- Tag filtering composes with existing cwd filtering semantics; users still need `--all` when they want to search beyond the current cwd.
- Invalid tag values and invalid tag filter patterns fail as usage errors instead of being silently accepted.
- A missing job passed to `tag set` returns the existing JSON error contract with `error.code = "job_not_found"`.

## Out of Scope

- Docker-style `key=value` labels.
- Arbitrary wildcard syntax beyond a trailing namespace `.*` suffix.
- OR semantics across repeated `list --tag` filters.
- Incremental mutation commands such as `tag add`, `tag remove`, or `tag clear`.
