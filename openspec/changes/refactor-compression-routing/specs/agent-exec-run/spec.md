## ADDED Requirements

### Requirement: Compression routing refactors preserve classification and summary contracts

Internal compression routing refactors MUST preserve route priority, detected-kind stable strings, supported compression modes, summary safety behavior, and raw observation compatibility. Classification responsibilities and summarization responsibilities MAY be reorganized internally only when externally observable compression behavior remains equivalent.

#### Scenario: classification priority remains stable

**Given**: an observed command/output could match multiple route heuristics such as repeated error-bearing logs and generic errors
**When**: route compression is applied after the refactor
**Then**: the same `compression.detected_kind` is selected as before
**And**: the same high-priority route family wins over lower-priority fallbacks

#### Scenario: summarization safety remains stable

**Given**: a routed compressor produces empty or non-smaller output for a non-empty raw stream
**When**: the compression response is built
**Then**: empty compressed output falls back to bounded summary where applicable
**And**: expansion guard suppresses oversized compressed output
**And**: canonical raw stdout/stderr fields remain unchanged
