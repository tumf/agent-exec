# Design: consolidate-log-metrics

## 方針

`run` の snapshot と `tail` の bytes メトリクス算出は同一の定義を持つため、`JobDir` に共通のヘルパーを追加して重複を除去する。JSON 形状や表示内容は変更せず、既存の仕様（`agent-exec-run`）に一致する値を返すことを最優先にする。

## 変更点（想定）

- `src/jobstore.rs`
  - 末尾取得と bytes メトリクスをまとめて返すヘルパー（例: `read_tail_metrics`）を追加
  - `tail_log_with_truncated` と `observed_bytes` の利用経路をこのヘルパーに集約
- `src/run.rs`
  - `build_snapshot` が新ヘルパーを使って `stdout`/`stderr` のメトリクスを取得
- `src/tail.rs`
  - `tail` の JSON 生成が新ヘルパー経由で `*_observed_bytes`/`*_included_bytes` を算出

## 互換性

- JSON 形状とキー名は不変
- `encoding="utf-8-lossy"` の固定値は維持
- 取得タイミングは現行のまま（`run` は snapshot 取得時点、`tail` は API 呼び出し時点）

## テスト方針

- 既存の統合テストとユニットテストをそのまま実行し、挙動の差分がないことを確認
