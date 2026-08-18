# Design: explicit Hermes completion delivery

## Boundary

`agent-exec` owns sink persistence and terminal dispatch. Hermes owns message routing and credentials. The adapter remains a shell helper under the bundled skill; Rust receives no Hermes dependency.

## Request-scoped target

The completion dispatcher (`dispatch_command_sink` in `src/run.rs`) executes the persisted `notify_command` string through the configured shell wrapper (e.g. `sh -lc`) and supplies only the canonical `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` variables. Managed-child `env` is not sink environment. Because the command runs under a shell, a leading environment assignment is valid; the caller therefore persists the destination with the command itself:

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

A fake `hermes` executable captures argv in an isolated temporary directory. The focused tests live in `tests/integration.rs` under a shared `hermes_notify_hook` name prefix and exercise the actual tracked helper, asserting exact arguments and fail-closed behavior. The missing-binary case must clear `HERMES_BIN` and isolate both `PATH` and `HOME`: the helper currently falls back to `$HOME/.hermes/hermes-agent/venv/bin/hermes`, so an unisolated `HOME` on a machine with a real Hermes installation would deliver a real message instead of failing. No real Slack, Telegram, credentials, network, or LLM is used.
