## ADDED Requirements

### Requirement: version-flag

The CLI must support `--version` and `-V` flags that print the crate version
and exit successfully.

#### Scenario: user-queries-version

**Given**: the `agent-exec` binary is built
**When**: the user runs `agent-exec --version`
**Then**: stdout contains a line matching `agent-exec <semver>`, and the
process exits with code 0

#### Scenario: short-flag-alias

**Given**: the `agent-exec` binary is built
**When**: the user runs `agent-exec -V`
**Then**: the output and exit code are identical to `agent-exec --version`
