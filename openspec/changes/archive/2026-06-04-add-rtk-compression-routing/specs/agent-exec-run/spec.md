## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

`run`/`start`/`restart`/`tail` は inline/tail 観測レスポンスに built-in compression view を追加できなければならない（MUST）。Compression は `--compress <mode>` または alias `--rtk <mode>` で制御できなければならず、外部 `rtk` コマンドを呼び出してはならない（MUST NOT）。Supported modes は `off|route|errors|tests|logs|git|json|summary` であり、`auto` を supported mode として受け付けてはならない（MUST NOT）。

Compression の built-in default は `route` でなければならない（MUST）。Config `[compression].default` が存在する場合は CLI 未指定時の既定 mode として使わなければならない（MUST）。Effective mode の優先順位は CLI `--compress`/`--rtk`、config `[compression].default`、built-in `route` でなければならない（MUST）。

Resolved mode が `off` 以外の場合、レスポンスは `compression` object を含まなければならない（MUST）。Resolved mode が `off` の場合、レスポンスは `compression` object を省略しなければならない（MUST）。Compression は canonical raw `stdout`/`stderr` fields または raw byte range fields を置換・変更してはならない（MUST NOT）。

Compression は、compressed view が対象 raw observed output より大きい、または同じ大きさになる場合、その compressed text をレスポンスへ含めてはならない（MUST NOT）。この expansion guard が発動した場合、レスポンスは bounded な `compression` object を含み、`compression.applied=false` と guard reason を示す strategy を返さなければならない（MUST）。

`route` compression は command argv と output shape から command family を分類できなければならない（MUST）。分類は外部 `rtk` command の実行やユーザー command の書き換えに依存してはならない（MUST NOT）。分類結果は `compression.detected_kind` に stable string として表現されなければならず、raw observation fields の互換性を壊してはならない（MUST NOT）。

#### Scenario: route compression classifies command family without command rewrite

**Given**: a command argv such as `git log --stat -30` is executed through `agent-exec run`
**When**: route compression is applied
**Then**: the response includes a command-family-specific `compression.detected_kind`
**And**: the original command argv is not rewritten to call an external `rtk` binary
**And**: canonical raw `stdout` and range fields still represent the observed command output

#### Scenario: expansion guard applies to routed compressors

**Given**: a routed command-family compressor produces a candidate that is greater than or equal to the raw observed output size
**When**: the response is built
**Then**: `compression.applied=false`
**And**: `compression.strategy` includes `expansion-guard`
**And**: the oversized candidate text is not embedded in `compression.stdout` or `compression.stderr`
