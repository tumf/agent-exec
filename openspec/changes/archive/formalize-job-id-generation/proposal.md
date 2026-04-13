---
change_type: hybrid
priority: medium
dependencies: []
references:
  - src/jobstore.rs
  - openspec/specs/agent-exec-jobstore/spec.md
---

# 変更提案: job_id 生成仕様の形式化と衝突リトライ上限

## Problem / Context

`generate_job_id` (`src/jobstore.rs:76-87`) は `JOB_ID_HEX_BYTES=16` で 32 文字 hex・`rand::thread_rng().fill_bytes`（128bit）を使うが衝突時は**無制限 loop**。spec には長さ・エントロピー源・上限が書かれていない。理論上は衝突しないが、誤ってディレクトリを手動作成された場合に無限ループする可能性がある。

## Proposed Solution

- spec: 「`job_id` は 32 文字の小文字 hex、128bit CSPRNG 由来、衝突時は最大 16 回まで再試行、超過時 `io_error` を返す」を MUST 化。
- 実装: 無制限 loop を 16 回上限に変更し、超過時 `anyhow::Error` を `io_error` にマッピング。

## Acceptance Criteria

- canonical spec に生成仕様と上限が明記される。
- 実装が 16 回試行で失敗する unit test が存在する（16 連続で同じ prefix の既存ディレクトリがある場合）。

## Out of Scope

- `job_id` 長さ変更や ULID 化。
