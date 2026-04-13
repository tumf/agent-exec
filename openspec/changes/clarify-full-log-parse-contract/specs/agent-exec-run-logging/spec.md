## ADDED Requirements

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
