## ADDED Requirements

### Requirement: restart preserves job identity while replacing the process

`agent-exec restart <job_id>` MUST reuse the existing job directory and persisted job definition while launching a fresh process for the same canonical `job_id`. Restart MUST NOT generate a new job id, move the job directory, or rewrite `meta.json.job.id`.

#### Scenario: restarting a terminal job keeps the same job id

**Given**: a job has reached a terminal state after running a persisted command
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the response has `ok=true`
**And**: the response has `type="restart"`
**And**: the response `job_id` equals the original job id
**And**: no additional job directory is created for the restart

#### Scenario: restarting a created job behaves like start

**Given**: a job exists in `created` state with a persisted command definition
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the persisted command is launched
**And**: the response has `type="restart"`
**And**: the response `job_id` equals the created job id

### Requirement: restart terminates running process before relaunch

When the target job is `running`, `agent-exec restart <job_id>` MUST terminate the currently associated process tree before launching the replacement process. Restart MUST NOT intentionally allow two concurrently running process trees for the same job id.

#### Scenario: running job is replaced

**Given**: a job is running a long-lived command
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the previously running process tree is terminated
**And**: a replacement process is launched using the same persisted job definition
**And**: subsequent `agent-exec status <job_id>` refers to the replacement run state

#### Scenario: restart does not relaunch when termination cannot be confirmed

**Given**: a job is running and its process tree cannot be terminated within the restart termination budget
**When**: `agent-exec restart <job_id>` is executed
**Then**: restart fails with a JSON error response
**And**: the command does not launch a second process for the same job id

### Requirement: restart returns run/start-compatible inline observation

`restart` MUST return a single JSON object on stdout and MUST use the same inline observation field contract as `run` and `start`: `job_id`, `state`, `tags`, `stdout_log_path`, `stderr_log_path`, `elapsed_ms`, `waited_ms`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, and `encoding`. Restart MUST support `--wait`, `--until`, `--forever`, `--no-wait`, and `--max-bytes` with semantics aligned to `start`.

#### Scenario: restart returns inline output fields

**Given**: a restartable job whose command prints `hello`
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: stdout is exactly one JSON object
**And**: the object has `type="restart"`
**And**: the object includes `stdout`, `stderr`, range fields, byte-total fields, and `encoding="utf-8-lossy"`
**And**: `stdout` includes `hello`

#### Scenario: restart no-wait returns promptly

**Given**: a restartable job whose command sleeps before producing output
**When**: `agent-exec restart --no-wait <job_id>` is executed
**Then**: the response returns without waiting for command completion
**And**: `waited_ms` is near zero

### Requirement: restart resets per-run output artifacts

Before launching the replacement process, restart MUST clear per-run output artifacts that would otherwise mix old and new run observations. At minimum, restart MUST truncate `stdout.log`, `stderr.log`, and `full.log`, and MUST remove or replace stale completion state for the previous run.

#### Scenario: restart output does not include stale stdout

**Given**: a job's `stdout.log` contains output from a previous run
**When**: `agent-exec restart <job_id> --wait` launches a new run
**Then**: the restart response stdout observation reflects the new run
**And**: `agent-exec tail <job_id>` does not include stale bytes from before restart

### Requirement: restart reuses persisted execution definition

Restart MUST launch from the persisted execution definition in `meta.json`, including command, cwd, env-file references, durable runtime env values, inherit-env setting, stdin file reference, tags, notification configuration, timeout settings, progress interval, and shell wrapper resolution. Restart MUST NOT require the caller to restate the command.

#### Scenario: restart reuses created job metadata

**Given**: a job was created with persisted stdin and environment metadata
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the child process receives the persisted stdin and environment configuration
**And**: masked values remain redacted in the JSON response

#### Scenario: restart rereads env files at launch time

**Given**: a job was created with an `--env-file` reference
**And**: the env file content changes after the original job definition was created
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the restarted process observes the current env file content
