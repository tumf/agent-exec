# Design: add-job-tags

## Overview

This proposal adds job tags at creation time, allows replacing tags on existing jobs, and adds list-time tag filtering. The design keeps tags as simple string metadata stored in `meta.json`, then reuses that persisted state for JSON responses and `list` filtering.

## Tag model

- `run --tag <TAG>` is repeatable.
- `tag set <JOB_ID> --tag <TAG>...` replaces the stored tags for an existing job.
- Stored tags are simple namespace-like strings such as `aaa` or `hoge.fuga.geho`.
- Tags are deduplicated before persistence, preserving first-seen order.
- Existing jobs without `meta.json.tags` remain readable and behave as having no tags.

`tag set` is the only post-creation mutation primitive in this proposal. It replaces the whole tag set rather than performing incremental add/remove operations.

The proposal intentionally does not introduce `key=value` labels, incremental tag editing, or arbitrary pattern syntax.

## Metadata update model

The repository already uses persisted job metadata as the source of truth for other workflow features. Tag mutation should follow the same approach:

- `run` writes the initial tag set into `meta.json`.
- `tag set` reloads `meta.json`, replaces `tags`, and writes the updated file atomically.
- `list` reads the current persisted tags, so updated jobs become discoverable without touching process state.

This keeps tagging independent from whether a job is currently running or already terminal.

## Pattern model for `list`

`list --tag <PATTERN>` is also repeatable and supports only two forms:

1. Exact match: the job has a tag equal to the pattern.
2. Namespace prefix match: the pattern ends with `.*`, and the job has a tag that is either exactly the namespace root or begins with `<namespace>.`.

Examples:

- `aaa` matches only `aaa`.
- `hoge.fuga.*` matches `hoge.fuga` and `hoge.fuga.geho`, but not `hoge.foo`.

Repeated `list --tag` filters use logical AND so callers can narrow to jobs carrying multiple independent tags.

## Filter composition

The repository already treats cwd filtering as the default `list` scope. Tag filtering should not change that contract.

The effective filtering order is:

1. Cwd filter (`current_dir`, `--cwd`, or `--all`)
2. Tag filters (`--tag`, AND semantics)
3. State filter (`--state`)
4. Sorting and `--limit`

This preserves the existing mental model while making tag filtering additive.

## Schema impact

- `meta.json` gains an optional `tags` array.
- `run` success responses gain `tags`.
- `tag set` success responses gain `tags`.
- `list.jobs[]` gains `tags`.

Backward compatibility is important because `list` already reads older jobs whose metadata may omit newer fields such as `cwd` or notification settings. Tag reads should therefore default missing `tags` to an empty list instead of treating old jobs as malformed.

## Validation boundary

The user request is specifically namespace-oriented (`hoge.fuga.geho` and `hoge.fuga.*`), so the design should validate toward that narrow surface:

- Stored tags allow dot-separated segments built from alphanumerics, `_`, and `-`.
- Filter patterns allow the same stored form plus a trailing `.*`.
- Mid-string wildcards such as `hoge.*.geho` and suffix-prefix forms such as `aaa*` are rejected.

That keeps matching predictable, cheap to implement, and easy to explain in CLI help and docs.
