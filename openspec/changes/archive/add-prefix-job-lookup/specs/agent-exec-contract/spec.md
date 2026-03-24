## ADDED Requirements

### Requirement: ambiguous_job_id error code

When a job ID prefix matches multiple job directories, the CLI MUST return a JSON error response with `error.code = "ambiguous_job_id"`, `error.retryable = false`, and exit code `1`. The `error.message` MUST include the ambiguous prefix and at least some of the matching candidate IDs to help the user disambiguate.

#### Scenario: ambiguous prefix error response

**Given**: Two or more jobs exist whose IDs share the same prefix `01JQXK`
**When**: `agent-exec status 01JQXK` is executed
**Then**: The exit code is `1`
**And**: The JSON response has `ok = false`
**And**: `error.code` is `"ambiguous_job_id"`
**And**: `error.retryable` is `false`
**And**: `error.message` contains the prefix and candidate job IDs
