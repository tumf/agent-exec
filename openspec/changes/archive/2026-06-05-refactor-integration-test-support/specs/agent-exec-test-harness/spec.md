## ADDED Requirements

### Requirement: Integration test support remains behavior-preserving

Integration-test support refactors MUST preserve isolated-root setup, JSON-only stdout assertions, usage-error expectations, stdin execution paths, and command contract coverage. Shared helpers MAY be extracted when existing tests continue to verify the same externally observable CLI behavior.

#### Scenario: isolated root execution remains consistent

**Given**: an integration test uses a temporary job root
**When**: the test runs `agent-exec` through shared test support
**Then**: `AGENT_EXEC_ROOT` or explicit root flags are applied as intended
**And**: job artifacts are created under the isolated root
**And**: tests do not leak state between cases

#### Scenario: JSON and usage-error assertions remain strict

**Given**: an integration test expects a successful JSON response or a clap usage error
**When**: the command is executed through shared test support
**Then**: successful stdout is parsed as exactly one JSON object with the expected envelope fields
**And**: usage errors assert exit code 2 with empty stdout
**And**: diagnostic stderr remains available in failure messages
