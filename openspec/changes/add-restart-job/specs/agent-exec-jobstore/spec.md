## ADDED Requirements

### Requirement: restart keeps job directory identity and definition files

Restart MUST preserve the existing `<root>/<job_id>/` directory and the job's canonical identity files. It MUST NOT delete or recreate the job directory as a different id. Restart MAY update `state.json` for the fresh run and MUST preserve `meta.json` identity and persisted execution definition.

#### Scenario: restart keeps the same job directory

**Given**: a job directory exists at `<root>/<job_id>/`
**When**: `agent-exec restart <job_id>` succeeds
**Then**: the same directory path remains the authoritative job directory
**And**: `meta.json.job.id` still equals `<job_id>`
**And**: `state.json.job.id` still equals `<job_id>`

### Requirement: restart clears per-run log files in place

Restart MUST clear per-run log files in place before launching the replacement process. The cleared files MUST remain at the canonical job directory paths so existing `tail`, `status`, and file path references continue to work.

#### Scenario: canonical log paths survive restart

**Given**: a job directory contains `stdout.log`, `stderr.log`, and `full.log`
**When**: `agent-exec restart <job_id>` succeeds
**Then**: those files still exist under the same job directory paths
**And**: their contents after restart correspond to the replacement run, not the previous run
