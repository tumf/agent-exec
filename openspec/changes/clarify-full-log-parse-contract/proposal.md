---
change_type: spec-only
priority: low
dependencies: []
references:
  - src/run.rs
  - openspec/specs/agent-exec-run-logging/spec.md
---

# 変更提案: full.log を人間向け表示用とし機械パース対象外であることを明記

## Problem / Context

`stream_to_logs` (`src/run.rs:905-958`) は `full.log` に `\n` 区切りで `from_utf8_lossy`（U+FFFD 置換）+ `"{ts} [{label}] {line}"` を書く。CR や非 UTF-8 バイトは lossy 置換され、長行も 1 行として扱う。spec にエスケープ規則が無いため、agent がパースすると壊れる。stdout.log / stderr.log は生バイト保存なので、機械処理はそちらを使うべき。

## Requested Artifact: spec/documentation

canonical 化のみ。

## Proposed Solution

- `agent-exec-run-logging/spec.md` に「`full.log` は人間向け表示用ログであり、機械パース対象ではない」「改行分割は `\n` のみ、CR および非 UTF-8 バイトは U+FFFD に lossy 置換」「機械処理は `stdout.log` / `stderr.log`（生バイト）を用いる」を MUST として明記。

## Acceptance Criteria

- canonical spec に full.log の扱いが明記される。
- agent skills/docs に「機械処理には stdout.log/stderr.log を使う」の記述が追加される（参照レベル）。

## Out of Scope

- full.log のフォーマット変更。
