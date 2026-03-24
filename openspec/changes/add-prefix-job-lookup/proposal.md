# Change Proposal: add-prefix-job-lookup

## Problem / Context

Job IDs are generated as ULIDs (26-character Crockford Base32 strings). In the
current era (2024-2026+), all ULIDs begin with `01`, making the first two
characters of every job ID identical. When users interact with `status`, `tail`,
`kill`, etc., they must type or paste the full 26-character ID, which is tedious
and error-prone.

Docker solved the same problem for container IDs: the full ID is always stored,
but users may specify an unambiguous prefix of any length. This is the approach
we will adopt.

## Proposed Solution

Add **prefix-based job ID resolution** to `JobDir::open` in
`src/jobstore.rs`. When the given string does not match a job directory
exactly, scan the root directory for directories whose name starts with the
given prefix. If exactly one match is found, resolve to that job. If zero
matches are found, return `JobNotFound`. If multiple matches are found, return
a new `AmbiguousJobId` error.

### Key design points

1. **No change to ID generation** -- ULIDs remain the canonical ID format.
2. **Exact-match fast path** -- when the user supplies the full ID, no
   directory scan occurs (zero performance regression).
3. **New sentinel error `AmbiguousJobId`** -- mirrors the existing
   `JobNotFound` pattern. Produces `error.code = "ambiguous_job_id"`,
   `retryable = false`, and lists candidate IDs in the message.
4. **All subcommands benefit automatically** -- every subcommand that takes a
   `job_id` argument goes through `JobDir::open`, so prefix lookup works
   everywhere without per-command changes.
5. **JSON responses always contain the resolved full ID** -- the `job_id`
   field in any response is the canonical ULID, never the user-supplied
   prefix.

## Acceptance Criteria

- [ ] `agent-exec status <prefix>` resolves to the correct job when the
      prefix is unambiguous (verified by integration test).
- [ ] `agent-exec status <ambiguous-prefix>` returns `error.code =
      "ambiguous_job_id"` with exit code 1 (verified by integration test).
- [ ] `agent-exec status <full-ulid>` continues to work as before (no
      regression).
- [ ] `agent-exec status <nonexistent>` returns `error.code =
      "job_not_found"` (existing behavior preserved).
- [ ] All other subcommands accepting `job_id` (`tail`, `wait`, `kill`,
      `start`, `tag set`, `notify set`, `delete`) also support prefix
      lookup (verified by at least one cross-command integration test).
- [ ] `prek run -a` passes (fmt, clippy, tests).

## Out of Scope

- Changing the ID generation scheme (ULID stays).
- Adding a minimum prefix length constraint (any length is accepted; ambiguity
  is handled by the error).
- UI/display changes to `list` output (e.g. short-ID column).
