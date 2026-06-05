## ADDED Requirements

### Requirement: CLI option groups preserve public command contracts

Internal refactoring of CLI option handling MUST preserve existing public flags, aliases, defaults, conflicts, completions, response schemas, and persisted metadata contracts. Shared option group helpers MUST NOT weaken the requirement that `run` and `create` share persisted definition-time inputs.

#### Scenario: shared definition metadata remains aligned

**Given**: `agent-exec create` and `agent-exec run` accept a persisted definition-time option such as tags, notification settings, environment settings, stdin settings, cwd, timeout, or shell wrapper
**When**: internal command dispatch is refactored through shared option group helpers
**Then**: both commands still accept the same persisted definition-time option where required
**And**: both commands persist equivalent metadata shape for that option
**And**: no stdout JSON envelope or persisted field name changes as a result of the refactor

#### Scenario: observation options remain command-specific where appropriate

**Given**: `run`, `start`, and `restart` support inline observation and compression options
**When**: those options are mapped through shared internal structures
**Then**: the existing defaults for waiting, byte limits, compression mode, and `--no-wait` behavior are preserved
**And**: commands that do not expose those options do not gain new public flags
