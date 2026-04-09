## Implementation Tasks

- [ ] Extend `src/serve.rs` wait handler to support bounded waiting with a default 30,000ms deadline plus `until_ms` / `forever` query parameters, while preserving the current JSON response schema (`verification: integration - new HTTP wait tests cover default deadline, explicit `until_ms`, and `forever` success cases`).
- [ ] Enforce mutual exclusion and request validation for `until_ms` and `forever` in the HTTP layer, returning stable HTTP 400 JSON errors for invalid combinations (`verification: integration - a `/wait/{id}?until_ms=100&forever=true` request returns HTTP 400 with a stable error code/message assertion`).
- [ ] Update `openspec/specs/agent-exec-serve/spec.md` and `README.md` so `/wait/{id}` is documented as the transport equivalent of CLI `wait`, including the 30,000ms default and query-parameter overrides (`verification: manual - endpoint docs and examples describe bounded waiting instead of unconditional blocking`).
- [ ] Add or update integration coverage so HTTP wait returns non-terminal state without killing the job when the deadline expires (`verification: integration - test asserts job remains runnable after HTTP wait timeout and can later be observed as terminal`).
- [ ] Run strict OpenSpec validation for this change after all proposal artifacts are authored (`verification: strict - `python3 /Users/tumf/.agents/skills/cflx-proposal/scripts/cflx.py validate align-serve-wait-deadline --strict``).

## Future Work

- Consider whether POST `/exec` should gain `wait_until_ms` / `wait_forever` request fields in a follow-up proposal so launch-and-wait and follow-up `/wait/{id}` are fully symmetric.
