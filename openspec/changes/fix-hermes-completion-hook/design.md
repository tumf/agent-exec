# Design: explicit Hermes completion delivery

## Boundary

`agent-exec` owns sink persistence and terminal dispatch. Hermes owns message routing and credentials. The adapter remains a shell helper under the bundled skill; Rust receives no Hermes dependency.

## Request-scoped target

The completion dispatcher intentionally supplies only canonical `AGENT_EXEC_*` event variables. Managed-child `env` is not sink environment. Therefore the caller persists the destination with the command itself:

```text
notify_command: "HERMES_NOTIFY_TARGET='slack:C0123:171234.0001' /absolute/path/hermes-notify-hook"
```

The helper refuses an absent target. It does not inspect Hermes state, cache a prior route, or infer a channel from process environment beyond the explicit assignment.

## Delivery semantics

The helper executes:

```text
hermes send --quiet --to <target> "job_id=<id> event_path=<path>"
```

A zero exit is command-delivery evidence only. A non-zero exit is recorded by the canonical command sink and does not alter workload terminal state.

## Verification

A fake `hermes` executable captures argv in an isolated temporary directory. The focused test exercises the actual tracked helper and asserts exact arguments and fail-closed behavior. No real Slack, Telegram, credentials, network, or LLM is used.
