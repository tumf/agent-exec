# 変更提案: run/tail の bytes メトリクスとログパス追加

## 背景

`run`/`tail` の出力に含まれるログ量を、bytes ベースで把握したい。
また `run` の待機は最大 10 秒に制限し、待機時間と全体所要時間を明示したい。

## 目的

- `run`/`tail` の JSON にログのフルパスを含める。
- `run` の `snapshot` と `tail` に bytes メトリクスを含める。
- `run` の待機を最大 10 秒に制限し、`waited_ms`/`elapsed_ms` を返す。

## スコープ

- `agent-exec run` と `agent-exec tail` の JSON 出力拡張
- `snapshot` の生成ロジック（bytes メトリクス追加）
- 仕様書の更新（agent-exec, agent-exec-run）

## 非スコープ

- トークンベースのサイズ制限
- `status`/`wait`/`kill` の挙動変更
- `stdout.log`/`stderr.log` の生成方式変更

## 仕様変更の要約

- `run` の `snapshot-after` 待機は最大 10,000ms にクランプする。
- `run` の JSON に `waited_ms` と `elapsed_ms` を追加する。
- `run`/`tail` の JSON に `stdout_log_path`/`stderr_log_path` を追加する。
- `snapshot` および `tail` に `*_observed_bytes` と `*_included_bytes` を追加する。

## 影響と互換性

- 追加フィールドのみで互換性は維持される。
- 既存の `truncated` 判定や `encoding` は保持する。

## 検証方針

- 統合テストに新フィールドの存在確認を追加する。
- `snapshot-after` が 10 秒にクランプされることを検証する。
