## ADDED Requirements

### Requirement: Prefix-based job ID resolution

`JobDir::open` MUST support prefix-based job ID resolution. When the given ID string does not match a job directory exactly, it MUST scan the root directory for directories whose name starts with the given string. If exactly one directory matches, it MUST resolve to that job. If no directory matches, it MUST return `JobNotFound`. If multiple directories match, it MUST return `AmbiguousJobId`.

The exact-match check MUST be attempted first, before any directory scan, so that full-ID lookups incur no additional I/O.

#### Scenario: Unique prefix resolves to the correct job

**Given**: A job with ID `01JQXK3M8E5PQRSTVWYZ12ABCD` exists under root
**When**: `JobDir::open(root, "01JQXK3M")` is called
**Then**: It returns a `JobDir` with `job_id = "01JQXK3M8E5PQRSTVWYZ12ABCD"`

#### Scenario: Exact full ID still works

**Given**: A job with ID `01JQXK3M8E5PQRSTVWYZ12ABCD` exists under root
**When**: `JobDir::open(root, "01JQXK3M8E5PQRSTVWYZ12ABCD")` is called
**Then**: It returns a `JobDir` with `job_id = "01JQXK3M8E5PQRSTVWYZ12ABCD"` without scanning the directory

#### Scenario: Non-existent prefix returns JobNotFound

**Given**: No job directory starts with `ZZZZZ` under root
**When**: `JobDir::open(root, "ZZZZZ")` is called
**Then**: It returns an error containing `JobNotFound`

#### Scenario: Ambiguous prefix returns AmbiguousJobId

**Given**: Jobs `01JQXK3M8EAAA` and `01JQXK3M8EBBB` exist under root
**When**: `JobDir::open(root, "01JQXK3M8E")` is called
**Then**: It returns an error containing `AmbiguousJobId` with both job IDs listed as candidates

### Requirement: AmbiguousJobId sentinel error

`AmbiguousJobId` MUST be a dedicated sentinel error type carrying the prefix string and a list of matching candidate job IDs. Its `Display` output MUST include the prefix and up to 5 candidate IDs. When more than 5 candidates match, it MUST summarize the remainder as `"... and N more"`.

#### Scenario: Display output with many candidates

**Given**: An `AmbiguousJobId` error with prefix `"01J"` and 8 candidate IDs
**When**: The error is formatted via `Display`
**Then**: The output includes the prefix, 5 candidate IDs, and `"... and 3 more"`

### Requirement: Resolved full ID in responses

All subcommands that accept a job ID argument and resolve it via prefix lookup MUST include the resolved full job ID (not the user-supplied prefix) in their JSON response `job_id` field.

#### Scenario: status with prefix returns full ID

**Given**: A job with full ID `01JQXK3M8E5PQRSTVWYZ12ABCD` exists
**When**: `agent-exec status 01JQXK3M` is executed
**Then**: The JSON response contains `"job_id": "01JQXK3M8E5PQRSTVWYZ12ABCD"`
