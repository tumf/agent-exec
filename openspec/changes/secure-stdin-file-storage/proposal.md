---
change_type: hybrid
priority: high
dependencies: []
references:
  - src/run.rs
  - src/schema.rs
  - openspec/specs/agent-exec-jobstore/spec.md
---

# 変更提案: stdin.bin を 0o600 + サイズ上限で保存

## Problem / Context

`--stdin` / `--stdin-file` で供給された入力は `stdin.bin` (`src/run.rs:194`) として job directory に保存される。現状 `File::create` の既定パーミッション（umask 依存、通常 0644）で、chmod 無し・サイズ上限無し。stdin はトークンや秘密鍵を含み得るため、ローカル FS 上の読み取り権限拡張は機密漏洩。サイズ無制限は DoS リスク。

## Proposed Solution

- 実装:
  - Unix 系で `stdin.bin` を `0o600` で作成（`OpenOptions::mode(0o600)`）。
  - 書き込みサイズ上限 64 MiB（設定可能: `--stdin-max-bytes`）、超過で `stdin_too_large` エラー。
- spec: canonical に上記を MUST 化。ファイル名は `stdin.bin` 固定（相対パス）、パーミッション 0o600、上限 64 MiB。

## Acceptance Criteria

- `agent-exec create --stdin "secret" -- cat` 実行後、`stdin.bin` のパーミッションが 0o600。
- 65 MiB の入力を投げると `stdin_too_large` エラー。
- Windows では ACL 既定を維持し、パーミッション要件は Unix のみ適用（spec で明示）。

## Out of Scope

- stdin 暗号化。
