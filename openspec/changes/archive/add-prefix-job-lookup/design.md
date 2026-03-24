# Design: add-prefix-job-lookup

## Overview

Enable Docker-style prefix-based job ID resolution so users can type only
the first few characters of a ULID job ID instead of the full 26-character
string.

## Resolution Algorithm (in `JobDir::open`)

```text
open(root, input) ->
  1. if root/input is a directory   -> return JobDir { input }        // exact match
  2. scan root/ for dirs starting with input
     a. 0 matches  -> Err(JobNotFound(input))
     b. 1 match    -> return JobDir { matched_name }                 // prefix resolved
     c. 2+ matches -> Err(AmbiguousJobId { prefix, candidates })     // ambiguous
```

### Exact-match fast path

Step 1 is a single `fs::metadata` call (or `path.exists()`), identical to
the current implementation. When the user supplies the full ID, prefix
scanning is never triggered. This preserves the current O(1) performance
for all internal callers (e.g. the supervisor, which always uses full IDs).

### Directory scan

Step 2 calls `fs::read_dir(root)` and filters entries by
`entry_name.starts_with(input)`. The scan is O(n) in the number of job
directories. This is acceptable because:

- Prefix lookup is only triggered when the exact path does not exist (i.e.
  user-facing commands with abbreviated IDs).
- Job stores are typically small (tens to low hundreds of jobs before GC).
- The scan touches directory metadata only, not file contents.

### AmbiguousJobId error

```rust
pub struct AmbiguousJobId {
    pub prefix: String,
    pub candidates: Vec<String>,
}
```

- `Display` shows up to 5 candidate IDs; additional matches are summarised
  as `"... and N more"`.
- Mapped to `error.code = "ambiguous_job_id"` in `main.rs`, with
  `retryable = false` and exit code 1.

## Affected Components

| Component | Change |
|-----------|--------|
| `src/jobstore.rs` | `AmbiguousJobId` struct + `JobDir::open` prefix logic |
| `src/main.rs` | Error-handling branch for `AmbiguousJobId` |
| `tests/integration.rs` | New tests for prefix resolve, ambiguous, cross-command |

No changes required in individual subcommand modules (`status.rs`,
`tail.rs`, `wait.rs`, `kill.rs`, `start.rs`, `tag.rs`, `notify.rs`,
`delete.rs`) because they all delegate to `JobDir::open`.

## Response Contract

JSON responses always contain the **resolved full ID** in `job_id`, never
the user-supplied prefix. This means downstream consumers never see
abbreviated IDs; the abbreviation is purely a CLI convenience.

## Minimum Prefix Length

No minimum is enforced. A 1-character prefix is valid; if it is ambiguous
the user receives `ambiguous_job_id` with candidate suggestions.
Rationale: adding a minimum would be an arbitrary constraint that provides
no safety benefit (ambiguity errors already prevent misidentification).
