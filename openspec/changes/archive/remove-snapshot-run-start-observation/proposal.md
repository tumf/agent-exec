---
change_type: implementation
priority: high
dependencies: []
references:
  - README.md
  - src/main.rs
  - src/run.rs
  - src/start.rs
  - src/schema.rs
  - src/wait.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: remove-snapshot-run-start-observation

**Change Type**: implementation

## Premise / Context
- このセッションでは、`agent-exec` のコアコンセプトは「non-interactive agent job runner」であり、`run` はジョブ起動、以後の観測は `status` / `wait` / `tail` に分離されるべきだという前提が共有された。
- 現行実装では `run` / `start` が `--snapshot-after`、`--tail-lines`、`--max-bytes`、`snapshot`、`final_snapshot` を持ち、`run --wait` 系とも意味が重なっている。
- canonical spec と README には `run` の default snapshot と `start` の snapshot/wait payload が広く記述されている一方、README 冒頭の Quick Start は `run -> wait -> tail` を基本導線としている。
- `tests/integration.rs` では多くのテストが `--snapshot-after 0` を「即時 return に戻すための回避策」として使っており、現行 default が本来の起動コマンドらしさから外れていることを示している。
- CLI 契約変更では `src/main.rs`・`src/schema.rs`・`tests/integration.rs`・README・OpenSpec を揃えて更新するのがこのリポジトリの前提である。

## Requested Artifact
- implementation
- `run` / `start` から snapshot 観測責務を除去し、観測は `wait` / `tail` / `status` に集約する変更提案を作る。

## 背景 / 課題
現行の `run` / `start` は、ジョブの起動に加えて snapshot 取得、返却前待機、完了時 tail 同梱まで担っており、ジョブランナーとしての責務分離を崩している。特に `--snapshot-after` は `run --wait --until/--forever` と意味領域が重なり、実装でも `--wait` 併用時に実質無効化されるなど、CLI 契約と利用者の期待がずれている。また default で 10 秒待機して snapshot を返す設計は、`run` を「まず起動して job_id を得る」コマンドとして使いたい呼び出し側に不要な遅延と複雑さを持ち込んでいる。

## 提案する変更
- `run` から `--snapshot-after`、`--tail-lines`、`--max-bytes` を削除する。
- `start` から `--snapshot-after`、`--tail-lines`、`--max-bytes` を削除する。
- `run` / `start` のレスポンスから `snapshot`、`final_snapshot`、snapshot 由来の `waited_ms` を削除する。
- `run` は job 起動後ただちに JSON を返すコマンドとして定義し直し、観測は `status` / `wait` / `tail` に委譲する。
- `start` も persisted job を起動して即時返すコマンドとして定義し直す。
- `run --wait` と `start --wait` はこの提案で廃止し、同期完了待機は `wait <job_id>` に一本化する。
- `tail` は引き続きログ末尾取得の唯一の観測 API とし、UTF-8 lossy / bytes メトリクス / ログパス契約は `tail` 側へ集約する。
- README の基本導線を `run -> wait -> tail` と `create -> start -> wait -> tail` に統一する。

## Acceptance Criteria
- `agent-exec run -- <cmd>` は snapshot 取得のために追加待機せず、job 起動後すぐに `job_id` と初期 state を返す。
- `agent-exec start <job_id>` も snapshot 取得のために追加待機せず、ジョブ起動後すぐに `job_id` と初期 state を返す。
- `agent-exec run --snapshot-after 10 -- <cmd>`、`agent-exec run --tail-lines 10 -- <cmd>`、`agent-exec run --max-bytes 10 -- <cmd>` は usage error になる。
- `agent-exec start --snapshot-after 10 <job_id>`、`agent-exec start --tail-lines 10 <job_id>`、`agent-exec start --max-bytes 10 <job_id>` は usage error になる。
- `agent-exec run --wait -- <cmd>` と `agent-exec start --wait <job_id>` は usage error になる。
- `run` / `start` の JSON には `snapshot` と `final_snapshot` が含まれない。
- 完了待機と出力観測は `wait <job_id>` と `tail <job_id>` で行う canonical 導線が spec と README に明記される。
- `tail` の UTF-8 lossy / bytes メトリクス / ログパス契約は維持される。

## Out of Scope
- `tail` / `wait` の JSON エンベロープ形式の全面刷新
- ログ保存方式 (`stdout.log` / `stderr.log` / `full.log`) の変更
- 通知機能や persisted definition metadata の仕様変更
- `wait` サブコマンド自体の削除や rename
