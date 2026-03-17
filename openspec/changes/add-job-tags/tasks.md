## Implementation Tasks

- [x] Add repeatable `--tag <TAG>` parsing for `run`, repeatable `--tag <PATTERN>` parsing for `list`, and `agent-exec tag set <JOB_ID> --tag <TAG>...` command parsing in `src/main.rs`, including validation rules for stored tags versus list filter patterns (verification: clap wiring and validation paths in `src/main.rs` accept repeated flags for `run`/`tag set`, reject invalid values, and dispatch the new subcommand).
- [x] Extend persisted and response schema types in `src/schema.rs` so `meta.json`, `run` responses, `tag set` responses, and `list.jobs[]` can carry tags while remaining backward-compatible for jobs created before the feature (verification: `JobMeta`, `RunData`, `JobSummary`, and the new `tag set` response type define `tags` with optional/default serde behavior that keeps old job metadata readable).
- [x] Implement tag deduplication and persistence in `src/run.rs`, ensuring first-seen order is preserved and `meta.json.tags` is written with the created job metadata (verification: `src/run.rs` constructs the persisted `JobMeta` and returned `RunData` from the deduplicated tag list).
- [x] Implement `tag set` metadata update logic that loads an existing job's `meta.json`, replaces `tags` atomically with the deduplicated requested list, and preserves unrelated metadata fields (verification: the new command updates only `meta.json.tags` and returns `job_not_found` for missing jobs).
- [x] Implement tag-pattern matching in `src/list.rs` so exact and trailing-`.*` namespace filters are applied with logical AND after cwd filtering and before limit truncation (verification: `src/list.rs` filters `jobs` using persisted tags and still respects existing cwd/state/limit semantics).
- [x] Add integration coverage in `tests/integration.rs` for repeatable `run --tag`, duplicate-tag deduplication, successful `tag set` replacement on existing jobs, missing-job errors for `tag set`, exact tag matching, namespace prefix matching, AND behavior across repeated `list --tag`, and composition with existing cwd filtering (verification: new tests fail without the feature and pass with it).
- [x] Update `README.md` usage for `run`, `tag set`, and `list` so the new tagging workflow and `hoge.fuga.*`-style filtering are documented with copy-pastable examples (verification: `README.md` includes tag assignment at creation time, post-creation replacement, and tag-filter examples).

## Future Work

- Consider a follow-up proposal for incremental tag management if workflows need `tag add`, `tag remove`, or `tag clear` commands in addition to `tag set`.
- Consider richer query operators only after repeatable exact/prefix filters prove insufficient.
