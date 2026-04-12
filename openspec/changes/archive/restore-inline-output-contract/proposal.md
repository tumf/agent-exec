---
change_type: implementation
priority: high
dependencies: []
references:
  - AGENTS.md
  - src/main.rs
  - src/run.rs
  - src/start.rs
  - src/tail.rs
  - src/schema.rs
  - src/serve.rs
  - tests/integration.rs
  - tests/serve_integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - openspec/changes/archive/remove-snapshot-run-start-observation/proposal.md
  - openspec/changes/archive/2026-02-18-include-run-output-default/proposal.md
  - openspec/changes/archive/2026-02-18-set-default-snapshot-after-10s/proposal.md
  - openspec/changes/archive/2026-02-19-add-run-wait/proposal.md
---

# 変更提案: restore-inline-output-contract

**Change Type**: implementation

## Premise / Context
- このセッションでユーザーは、`agent-exec` の主用途を AI エージェントの往復削減と明示し、`run` / `start` の初回レスポンスに短命コマンドの結果が含まれるべきだと要求している。
- 現行 canonical spec と実装は `run` / `start` を launch-only とし、`wait` / `tail` へ観測責務を分離している (`src/run.rs`, `tests/integration.rs:1565`, `openspec/specs/agent-exec-run/spec.md:18`, `openspec/specs/agent-exec/spec.md:38`)。
- 履歴上は `include-run-output-default`・`set-default-snapshot-after-10s`・`add-run-wait` が存在し、現在の launch-only 仕様は `remove-snapshot-run-start-observation` で後から導入された。
- ユーザーは現在の launch-only 化をデグレと見なしており、`run` / `start` のデフォルト待機を 10 秒へ戻し、`--no-wait` を `--wait --until 0` のエイリアスとして追加したい。
- 出力表現は `snapshot` / `final_snapshot` / `truncated` を使わず、top-level の `stdout` / `stderr` と raw byte range `[begin, end]`・`*_total_bytes` で統一し、`run` / `start` は head、`tail` は tail を返す方針がこのセッションで合意された。
- `serve` の `POST /exec` は `run` 相当、`GET /tail/:id` は `tail` 相当なので、CLI 契約変更に追随させる必要がある (`openspec/specs/agent-exec-serve/spec.md:33`, `src/serve.rs:200`, `src/serve.rs:337`)。

## Requested Artifact
- implementation
- `run` / `start` / `tail` / `serve` の出力契約を range 表現へ統一し、`run` / `start` の既定 10 秒待機と初回 inline output をデグレ修正として復旧する。

## 背景 / 課題
現行の `run` / `start` は job 起動メタデータだけを即時返し、出力確認には別途 `wait` や `tail` の呼び出しが必要である。これは AI エージェントが短命コマンドの結果を読むだけでも複数往復を強いられる設計であり、ユーザーが期待する「1 回の `run` / `start` で状況判断できる」契約から後退している。さらに旧来の `snapshot` / `final_snapshot` / `stdout_tail` 命名は用途を誤解させやすく、head と tail の違いも API から読み取りにくい。

## 提案する変更
- `run` と `start` のデフォルト観測モードを `--wait --until 10` 相当に戻し、10 秒以内に観測できた stdout / stderr を初回レスポンスへ含める。
- `--no-wait` を新設し、`run` / `start` の `--wait --until 0` エイリアスとして定義する。
- `run` / `start` のレスポンスは top-level の `stdout` / `stderr` を返し、これは各ログの先頭 `N` bytes を UTF-8 lossy で表現する。
- `tail` も `stdout` / `stderr` を返し、こちらは各ログの末尾 `N` bytes / `tail-lines` 制約で得た部分を返す。
- `run` / `start` / `tail` の出力メタデータは `stdout_range` / `stderr_range` と `stdout_total_bytes` / `stderr_total_bytes` に統一し、`truncated` や `*_observed_bytes` / `*_included_bytes` は外部契約から外す。
- `stdout_range` / `stderr_range` は raw byte offset の `[begin, end]` 配列として返し、意味は half-open interval `[begin, end)` とする。
- `run` / `start` の終端時は `state`, `exit_code`, `finished_at`, `stdout`, `stderr`, range 情報を同じレスポンスに含め、非終端時もその時点まで観測できた head と `state=running|created` を返す。
- `serve` の `POST /exec` と `GET /tail/:id` は CLI と同じ出力フィールド名・range 契約に追従させる。
- canonical spec、テスト、README/skills/help から launch-only 前提と `snapshot` 命名を除去し、新しい inline output 契約へ揃える。

## Acceptance Criteria
- `agent-exec run -- <cmd>` と `agent-exec start <job_id>` は既定で最大 10 秒待機し、少なくとも `waited_ms`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding`, `stdout_log_path`, `stderr_log_path` を返す。
- 10 秒以内に終了する短命コマンドでは、追加の `wait` / `tail` を呼ばなくても初回レスポンスだけで `state`, `exit_code`, `stdout`, `stderr` を確認できる。
- `run` / `start` の `stdout_range[0]` と `stderr_range[0]` は 0 であり、返却内容は log 先頭 `N` bytes の UTF-8 lossy 表現になる。
- `agent-exec run --no-wait -- <cmd>` と `agent-exec start --no-wait <job_id>` は `--wait --until 0` と同義で、追加待機なしに返る。
- `agent-exec tail <job_id>` と `GET /tail/:id` は `stdout` / `stderr` と range 情報を返し、返却内容は log 末尾側の部分を表す。
- `stdout_range` / `stderr_range` と `*_total_bytes` があれば、呼び出し側が先頭欠落・末尾欠落・全量返却を機械的に判定できる。
- `snapshot` / `final_snapshot` / `truncated` / `stdout_tail` / `stderr_tail` / `*_observed_bytes` / `*_included_bytes` は新契約の canonical field 名として残らない。
- `POST /exec` は CLI の `run` と同じ既定待機・field 名を採用し、`GET /tail/:id` も CLI の `tail` と同じ shape を返す。

## Out of Scope
- ジョブ実行・監視の内部アーキテクチャ刷新
- ログ保存形式 (`stdout.log`, `stderr.log`, `full.log`) 自体の変更
- 新しい follow/streaming API の追加
- `status` / `wait` の JSON 契約全面刷新
