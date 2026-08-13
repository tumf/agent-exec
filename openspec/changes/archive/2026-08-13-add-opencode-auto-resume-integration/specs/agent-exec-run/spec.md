## ADDED Requirements

### Requirement: Optional client auto-resume integrations use existing notification contracts

The repository MAY provide opt-in reference integrations that connect managed-job completion to an originating AI-agent client session. Such integrations MUST use the existing persisted notification and completion-event contracts without changing core `agent-exec` runtime defaults or coupling MCP schemas to a specific client. Reference integrations MUST be portable, explicitly installed, locally testable without external credentials, and documented as best-effort client adapters rather than guaranteed delivery mechanisms.

#### Scenario: OpenCode plugin attaches completion routing after MCP run

**Given**: an explicitly installed OpenCode reference plugin observes a successful agent-exec MCP run tool result
**When**: the result contains a valid job ID and the originating OpenCode server and session pass the integration's local validation
**Then**: the integration attaches a completion callback through the existing persisted notification configuration
**And**: the agent does not need to provide notification arguments
**And**: the MCP run schema and core runtime behavior remain unchanged

#### Scenario: Short job does not resume the client session

**Given**: the OpenCode reference integration attached a completion callback to a managed job
**When**: the persisted completion event reports a duration below the configured minimum
**Then**: the callback exits successfully without adding an automation message to the OpenCode session

#### Scenario: Eligible job resumes the originating session once

**Given**: a managed job reaches terminal state at or above the configured minimum duration
**When**: the completion callback receives a valid loopback OpenCode target and originating session ID
**Then**: it submits one clearly machine-authored continuation prompt to that session
**And**: duplicate delivery for the same job does not submit a second successful continuation
**And**: completion event and log content are identified as untrusted data

#### Scenario: Invalid callback context is rejected safely

**Given**: a plugin result or completion callback contains a malformed job ID, malformed session ID, non-loopback server URL, malformed completion event, or unrelated tool output
**When**: the reference integration evaluates the input
**Then**: it does not invoke an unintended OpenCode session
**And**: it does not evaluate job output or event content as shell commands

#### Scenario: Reference integration installs without machine-specific assumptions

**Given**: a user has compatible `agent-exec` and OpenCode executables
**When**: the user follows the bundled installation documentation
**Then**: the plugin and helper can be installed using documented configurable locations
**And**: no tumf-specific path, fixed port, credential, or project repository is required
