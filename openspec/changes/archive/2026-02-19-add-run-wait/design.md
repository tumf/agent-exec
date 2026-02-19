# Design: add-run-wait

## 目的
`run --wait` を追加し、短命ジョブの「起動→完了待ち→最終ログ取得」を1レスポンスで返せるようにする。

## 主要な設計判断
- **互換性優先**: 既存 `run` のレスポンスは維持し、`--wait` 指定時のみ追加フィールドを返す。
- **最終ログは別フィールド**: 既存 `snapshot` の意味を保持するため、終了時点の末尾は `final_snapshot` として追加する。
- **待機上限の扱い**: `--wait` 指定時は `snapshot-after` の 10,000ms 制限を適用せず、終端状態まで待機する。

## レスポンス形
- `run --wait` の成功レスポンスに以下を追加
  - `exit_code` (存在する場合)
  - `finished_at`
  - `final_snapshot` (既存 `snapshot` と同一構造)
  - `waited_ms` は終端状態までの待機時間
- `state` は `exited|killed|failed` の終端状態を返す

## 失敗時の扱い
- 既存のエラーエンベロープ (`ok=false`, `error`) を維持
- `--timeout`/`--kill-after` により強制終了した場合は、終端状態として `killed` を返し `finished_at` を含める

## 実装の影響範囲
- CLI: `src/main.rs` に `--wait` フラグを追加
- Schema: `src/schema.rs` に `RunData` の追加フィールドを追加
- 実行: `src/run.rs` で待機ロジックを `wait` と共有する
- テスト: `tests/integration.rs` に `run --wait` のシナリオ追加
