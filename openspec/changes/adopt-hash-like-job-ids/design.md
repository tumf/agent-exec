# Design: hash-like job IDs

## Summary

`agent-exec` の新規 job ID を ULID から固定長の小文字 hex ランダム ID へ変更し、人間の運用では先頭 7 文字を `short_job_id` として扱う。完全 ID は job directory 名・永続メタデータ・API の canonical identifier として残し、CLI / HTTP の job lookup は exact match 優先・一意 prefix 解決を維持する。

## Goals

- 先頭数文字で job を識別しやすくする
- Docker / Git hash のような prefix 指定 UX を提供する
- 既存 ULID job directory を破壊せず混在運用する
- 現行の directory-based job store を維持する

## Non-Goals

- 古い job ID の rename/migration
- prefix 解決アルゴリズムの別方式化
- hash 以外の可読 alias レイヤ追加

## ID format

- 新規生成 ID は `[0-9a-f]` のみで構成する
- 長さは固定長とする
- 表示用 `short_job_id` は先頭 7 文字とする

長さの最終値は実装時に固定するが、proposal の受け入れ基準は「7文字短縮表示が常用 UX を満たすだけの十分な固定長 hex ID」であることを要求する。

## Generation strategy

1. 共通の `generate_job_id(root)` 相当ヘルパーを用意する
2. 暗号学的または十分なランダムソースから固定長 hex を作る
3. `root/<job_id>` が既に存在する場合は再生成する
4. 生成器は `run` / `create` / `serve /exec` から共通利用する

## Lookup compatibility

`JobDir::open` は既に directory 名ベースで exact match と prefix scan を行う。ここでは ID 形式に依存した解釈を追加せず、directory 名を opaque identifier として扱い続ける。

これにより:
- 既存 ULID job もそのまま参照可能
- 新形式 hash-like ID も同じ API で参照可能
- exact match 優先 / prefix fallback / ambiguous error の既存 contract を維持できる

## API / schema impact

### CLI / JSON

`list.jobs[]` に `short_job_id` を追加する。完全 ID の `job_id` は引き続き canonical field として保持する。

他コマンドは既存の `job_id` field を維持するが、proposal の主目的は「生成形式変更」と「一覧での短縮表示」であり、まずは `list` を最小対象とする。

### HTTP

HTTP endpoint の path parameter は完全 ID または一意 prefix を受け付ける。path 文字列の validation を新形式に限定してはならず、既存 ULID も受理できる必要がある。

## Testing strategy

- unit: 生成器のフォーマット、衝突再試行、jobstore の新旧混在 prefix 解決
- integration: `run` / `create` / `start` / `list` の JSON contract 更新
- integration: serve endpoint の新形式 ID / prefix 指定
- manual: 必要なら shell completion の視認性確認

## Risks

- `job_id` の文字列形式を暗黙に ULID とみなすテストや補助コードが壊れる可能性
- tie-breaker として `job_id` の辞書順を利用する箇所は、時系列の意味を持たなくなる
- 7 文字 prefix だけでは稀に衝突しうるため、常に「一意な prefix なら受理」という contract を明示する必要がある

## Migration / rollout

- 既存 job directory は変更しない
- 新規 job 作成分のみ新形式へ移行する
- 読み取り系・操作系は新旧両対応のままリリースする
