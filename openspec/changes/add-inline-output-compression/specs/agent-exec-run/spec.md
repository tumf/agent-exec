## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

`run`/`start`/`restart`/`tail` は inline/tail 観測レスポンスに built-in compression view を追加できなければならない（MUST）。Compression は `--compress <mode>` または alias `--rtk <mode>` で制御できなければならず、外部 `rtk` コマンドを呼び出してはならない（MUST NOT）。Supported modes は `off|route|errors|tests|logs|git|json|summary` であり、`auto` を supported mode として受け付けてはならない（MUST NOT）。

Compression の built-in default は `route` でなければならない（MUST）。Config `[compression].default` が存在する場合は CLI 未指定時の既定 mode として使わなければならない（MUST）。Effective mode の優先順位は CLI `--compress`/`--rtk`、config `[compression].default`、built-in `route` でなければならない（MUST）。

Resolved mode が `off` 以外の場合、レスポンスは `compression` object を含まなければならない（MUST）。Resolved mode が `off` の場合、レスポンスは `compression` object を省略しなければならない（MUST）。Compression は canonical raw `stdout`/`stderr` fields または raw byte range fields を置換・変更してはならない（MUST NOT）。

#### Scenario: tail が末尾観測 API である

**Given**: 実行中または完了済みのジョブが存在する
**When**: `agent-exec tail --tail-lines 10 --max-bytes 1024 <job_id>` を実行する
**Then**: `stdout`/`stderr` と `encoding="utf-8-lossy"` が返る
**And**: `stdout_range`/`stderr_range` が返る

#### Scenario: default route compression is included

**Given**: config に `[compression].default` が設定されていない
**When**: `agent-exec run -- sh -c "printf 'error: bad\\n'"` を実行する
**Then**: stdout は JSON-only の単一 object である
**And**: レスポンスは `compression.mode = "route"` を含む
**And**: canonical `stdout` と `stdout_range` は raw head 観測を表す

#### Scenario: rtk alias behaves like compress

**Given**: 同じ job output を生成するコマンドがある
**When**: `agent-exec run --compress errors -- <cmd>` と `agent-exec run --rtk errors -- <cmd>` を実行する
**Then**: 両レスポンスの effective compression mode は `errors` である
**And**: 両レスポンスは同じ compression strategy family を使う

#### Scenario: conflicting compression flags are rejected

**Given**: `--compress errors` と `--rtk logs` が同時に指定される
**When**: `agent-exec run --compress errors --rtk logs -- echo hi` を実行する
**Then**: コマンドは usage error として終了コード 2 で失敗する

#### Scenario: config default can disable compression

**Given**: config file に `[compression] default = "off"` が設定されている
**When**: その config を使って `agent-exec run -- echo hi` を実行する
**Then**: レスポンスは canonical raw observation fields を含む
**And**: レスポンスは `compression` object を含まない

#### Scenario: CLI compression mode overrides config default

**Given**: config file に `[compression] default = "off"` が設定されている
**When**: その config を使って `agent-exec run --compress route -- echo hi` を実行する
**Then**: レスポンスは `compression.mode = "route"` を含む

#### Scenario: invalid config compression mode fails with structured error

**Given**: config file に `[compression] default = "auto"` が設定されている
**When**: その config を使って `agent-exec run -- echo hi` を実行する
**Then**: stdout は JSON-only の error response である
**And**: `error.code` は config validation failure を示す stable code である

#### Scenario: off mode preserves compatibility shape

**Given**: `agent-exec run --compress off -- echo hi` を実行する
**When**: コマンドが返る
**Then**: レスポンスは canonical raw observation fields を含む
**And**: レスポンスは `compression` object を含まない

#### Scenario: compressed output does not replace raw output

**Given**: コマンドが repeated log lines と error lines を出力する
**When**: `agent-exec run --compress logs -- <cmd>` を実行する
**Then**: canonical `stdout` または `stderr` は raw head 観測を含む
**And**: `compression.stdout` または `compression.stderr` は repeated lines を集約した compact view を含む
**And**: `stdout_range` と `stderr_range` は raw byte range を表す
