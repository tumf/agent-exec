---
change_type: implementation
priority: medium
dependencies: []
references:
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - openspec/specs/agent-exec-jobstore/spec.md
  - src/run.rs
  - src/create.rs
  - src/serve.rs
  - src/jobstore.rs
  - src/completions.rs
  - src/list.rs
---

# Adopt hash-like job IDs

**Change Type**: implementation

## Premise / Context

- 現在の新規 `job_id` は `Ulid::new().to_string()` で生成され、`run`/`create`/`serve /exec` の全経路で ULID に固定されている。
- ULID は現在時刻帯ではほぼ常に `01` から始まるため、短い prefix による識別性と shell 補完の実用性が低い。
- 既存実装は `JobDir::open` で exact match 優先・一意 prefix 解決・曖昧時 `ambiguous_job_id` を提供しており、Docker 風の短い prefix 指定 UX と整合する。
- ユーザー要望は「git のコミットハッシュ / Docker ID のような形」で、常用表示は最初の 7 文字、指定は一意な先頭 prefix でよい、というもの。
- canonical spec は `job_id` の存在と job directory への保存は規定しているが、ID 文字列形式は明示固定していない。

## Problem / Context

現行の ULID ベース `job_id` は時刻由来の共通接頭辞を持つため、短い入力では候補がほとんど分岐せず、人間の視認・補完・prefix 指定の UX が悪い。`agent-exec` は日常運用で job ID を手入力・補完・会話中に参照するユースケースが多く、Docker コンテナ ID のように「短い prefix で大抵識別できる」形式へ移行する必要がある。

## Proposed Solution

新規 job 作成時の `job_id` 生成を ULID から固定長の小文字 hex ランダム ID へ変更する。`run`、`create`、HTTP `POST /exec` は同じ生成器を共有し、同一 root 配下で directory 名衝突が起きないよう再試行する。

同時に、人間向け表示用として `short_job_id`（完全 ID の先頭 7 文字）を定義し、`list` などの一覧系レスポンスで返す。job を受け取る各コマンドと HTTP endpoint は既存どおり exact match → 一意 prefix → ambiguous error の解決順を維持し、新旧 ID 形式を混在運用できるようにする。

## Acceptance Criteria

1. 新規作成される job は `01...` で始まる ULID ではなく、git hash / Docker ID 風の小文字 hex `job_id` を持つ。
2. `status` / `tail` / `wait` / `kill` / `start` / `delete` と対応 HTTP endpoint は、新形式 ID に対して一意な先頭 prefix 指定を受け付ける。
3. 既存の ULID job directory は引き続き読み取り・操作できる。
4. `list` の各 job summary は完全 `job_id` に加えて、常用表示向けの 7 文字 `short_job_id` を返す。
5. proposal に沿った実装後、統合テストで新形式生成・短縮表示・新旧混在 prefix 解決・HTTP 互換が検証される。

## Out of Scope

- 既存 job directory 名の一括マイグレーション
- `job_id` の途中一致や suffix 一致の導入
- shell completion エンジンそのものの全面刷新
