## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

`run`/`start`/`restart`/`tail` は inline/tail 観測レスポンスに built-in compression view を追加できなければならない（MUST）。Compression は `--compress <mode>` または alias `--rtk <mode>` で制御できなければならず、外部 `rtk` コマンドを呼び出してはならない（MUST NOT）。Supported modes は `off|route|errors|tests|logs|git|json|summary` であり、`auto` を supported mode として受け付けてはならない（MUST NOT）。

Compression の built-in default は `route` でなければならない（MUST）。Config `[compression].default` が存在する場合は CLI 未指定時の既定 mode として使わなければならない（MUST）。Effective mode の優先順位は CLI `--compress`/`--rtk`、config `[compression].default`、built-in `route` でなければならない（MUST）。

Resolved mode が `off` 以外の場合、レスポンスは `compression` object を含まなければならない（MUST）。Resolved mode が `off` の場合、レスポンスは `compression` object を省略しなければならない（MUST）。Compression は canonical raw `stdout`/`stderr` fields または raw byte range fields を置換・変更してはならない（MUST NOT）。

Compression は、compressed view が対象 raw observed output より大きい、または同じ大きさになる場合、その compressed text をレスポンスへ含めてはならない（MUST NOT）。この expansion guard が発動した場合、レスポンスは bounded な `compression` object を含み、`compression.applied=false` と guard reason を示す strategy を返さなければならない（MUST）。

#### Scenario: json compression applies when shape summary is smaller than raw output

**Given**: a command emits a JSON object whose raw observed output is larger than the JSON shape summary
**When**: `agent-exec run --compress json -- <cmd>` is executed
**Then**: the response includes `compression.applied=true`
**And**: `compression.stdout` contains an object shape summary such as `object keys=2`
**And**: canonical raw `stdout` still contains the original JSON output

#### Scenario: json compression guard suppresses non-smaller shape summary

**Given**: a command emits a short JSON object whose JSON shape summary would be greater than or equal to the raw observed output size
**When**: `agent-exec run --compress json -- <cmd>` is executed
**Then**: the response includes `compression.applied=false`
**And**: `compression.strategy` includes `expansion-guard`
**And**: `compression.stdout` is empty
**And**: canonical raw `stdout` still contains the original JSON output
