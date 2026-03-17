# Change Proposal: align-create-run-definition-options

## Problem/Context

`add-create-start-lifecycle` introduces `create` as the durable job-definition step and keeps `run` as the immediate-start convenience path, but the repository does not yet state a durable rule for how their job-definition options should relate over time.

- The current lifecycle design already says `run` should reuse the same lifecycle primitives as `create` + `start`.
- The user clarified that this should not be treated as a one-off for tags or notifications: `create` and `run` should continue to accept the same option set for the job-creation portion of the workflow.
- The immediate concrete gap is that tags and notification settings are already being proposed as durable metadata, yet their support on `create` can drift from `run` unless the broader contract is made explicit.
- In this repository, durable job definition is stored in `meta.json`, while observation-time behavior such as snapshot and wait belongs to launch-time or status-style commands.

## Proposed Solution

Define a shared rule that `create` and `run` must accept the same definition-time options for the job-creation portion of the lifecycle, while allowing `run` alone to keep immediate-execution and observation options.

- Treat `create` as the canonical metadata-only entrypoint for job definition and `run` as `create + immediate start`.
- Require every durable job-definition option added in the future to be considered for both `create` and `run` by default.
- Keep observation-time options scoped appropriately: `run` and `start` may expose snapshot, wait, and other launch-observation controls that `create` does not accept.
- Preserve one persisted metadata model so jobs created via `create` and jobs created via `run` remain equivalent except for whether execution starts immediately.
- Make tags, completion notification settings, and output-match notification settings the first concrete definition-time options covered by this rule.
- Require `create` to persist those concrete metadata families without side effects, and require `start` to consume the saved values when launching the job.

This remains a single proposal because it defines one lifecycle contract: the boundary between definition-time options and launch/observation-time options, plus the first concrete metadata families that must follow that contract.

## Acceptance Criteria

- The specs define a durable rule that `create` and `run` share the same definition-time option surface for persisted job metadata.
- Adding a new persisted job-definition field requires keeping `create` and `run` aligned unless the field is explicitly documented as launch-only.
- `agent-exec create --tag aaa --tag bbb -- <cmd>` succeeds and persists deduplicated tags in the same metadata shape used by `run`.
- `agent-exec create --notify-command 'cat >/tmp/event.json' -- <cmd>` succeeds, saves completion notification metadata, and does not execute the command during `create`.
- `agent-exec create --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json' -- <cmd>` succeeds, saves output-match notification metadata, and does not trigger delivery during `create`.
- Snapshot, wait, and similar observation options remain documented as launch-time behavior rather than `create` metadata.
- Jobs produced by `create` and by `run` persist equivalent `meta.json` fields for the shared definition-time options.
- `agent-exec start <job_id>` uses the tags and notification settings saved by `create` unless they were later changed through metadata mutation commands.

## Out of Scope

- Requiring `create` to accept observation-only options such as `--wait`, `--snapshot-after`, or tail sizing controls.
- Forcing `start` to accept the full definition-time surface rather than consuming the saved definition.
- Changing existing event payloads, tag filtering rules, or unrelated CLI contracts outside the create/run option-alignment rule.
