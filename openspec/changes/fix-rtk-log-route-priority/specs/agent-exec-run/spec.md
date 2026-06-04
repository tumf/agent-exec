## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

`run`/`start`/`restart`/`tail` は inline/tail 観測レスポンスに built-in compression view を追加できなければならない（MUST）。Compression は `--compress <mode>` または alias `--rtk <mode>` で制御できなければならず、外部 `rtk` コマンドを呼び出してはならない（MUST NOT）。Supported modes は `off|route|errors|tests|logs|git|json|summary` であり、`auto` を supported mode として受け付けてはならない（MUST NOT）。

System, search, log, JSON, and env-like outputs routed through `route` compression must use structure-aware compact views when recognized (MUST). Compression must group large listings and search results, deduplicate repetitive logs, summarize JSON structure without large values, and mask secret-like values in env-like compressed views (MUST). Compression must not mutate or replace canonical raw observation fields (MUST NOT).

Route compression must classify repeated or timestamp-normalized repeated log output as `logs` before falling back to generic `errors`, even when the repeated log message contains error-bearing words such as `ERROR` (MUST). Non-repeated one-off error, panic, traceback, or assertion output must continue to classify as `errors` when no stronger command-family or output-shape route matches (MUST).

#### Scenario: repeated timestamped error logs route to logs

**Given**: a command emits many timestamp-varied log lines with the same error message
**When**: `agent-exec run --rtk route -- <cmd>` observes that output
**Then**: `compression.detected_kind` is `logs`
**And**: `compression.strategy` includes `dedupe-normalized-log-lines`
**And**: the compressed stdout is substantially smaller than the raw observed stdout
**And**: canonical raw `stdout` and range fields still represent the original observed output

#### Scenario: single non-repeated error remains errors

**Given**: a command emits a single non-repeated error line
**When**: route compression is applied
**Then**: `compression.detected_kind` is `errors`
**And**: the error-bearing line remains available through the compression view when it is smaller than raw output or through canonical raw fields otherwise

#### Scenario: explicit errors mode remains available

**Given**: repeated error-bearing log output exists
**When**: `agent-exec run --rtk errors -- <cmd>` is executed
**Then**: the effective compression mode is `errors`
**And**: route-priority heuristics do not override the explicit user-selected mode
