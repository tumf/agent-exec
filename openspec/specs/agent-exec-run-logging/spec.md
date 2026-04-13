# agent-exec-run-logging Specification

## Purpose
TBD - created by archiving change refactor-run-log-streaming. Update Purpose after archive.
## Requirements
### Requirement: stdout/stderr ログ内容の互換

`run` が生成する `stdout.log` と `stderr.log` は、子プロセスの出力バイト列を順序どおりに保存しなければならない（MUST）。リファクタにより内容・順序・欠落が変化してはならない（MUST）。

#### Scenario: 連続出力の保存
Given `agent-exec run -- <cmd>` を実行し、`<cmd>` が stdout と stderr にそれぞれ複数行出力する
When コマンドが `run` の JSON を返す
Then `stdout.log` と `stderr.log` には出力と同じ順序・内容が保存される

### Requirement: full.log の行フォーマット互換

`full.log` の行は `<RFC3339> [STDOUT] <line>` または `<RFC3339> [STDERR] <line>` の形式で記録されなければならない（MUST）。リファクタによりこの形式が変わってはならない（MUST）。

#### Scenario: full.log の行形式
Given `agent-exec run -- <cmd>` を実行し、`<cmd>` が stdout と stderr に 1 行ずつ出力する
When `full.log` を読む
Then 各行が `RFC3339` 形式のタイムスタンプと `[STDOUT]` / `[STDERR]` プレフィックスを含む

## Requirements

### Requirement: full.log の利用契約

`full.log` は人間向けの混合ビュー（stdout と stderr を時系列順で並べたもの）であり、機械パースの対象ではない（MUST NOT）。行分割は `\n` のみで行い、CR (`\r`) は通常文字として保持する（MUST）。非 UTF-8 バイトは U+FFFD に lossy 置換する（MUST）。

機械処理（パース、差分比較、フィルタ）を行うエージェント／クライアントは `stdout.log` と `stderr.log`（いずれも生バイト保存）を用いなければならない（MUST）。

#### Scenario: full.log replaces non-UTF-8 bytes

**Given**: a command that emits a stray `\xff` byte to stdout
**When**: the supervisor writes `full.log`
**Then**: the line contains U+FFFD in place of `\xff`
**And**: `stdout.log` contains the original `\xff` byte

#### Scenario: stdout.log preserves binary bytes

**Given**: a command that emits binary bytes including `\r\n` and `\xff`
**When**: the supervisor writes `stdout.log`
**Then**: the file contains the exact byte sequence without translation
