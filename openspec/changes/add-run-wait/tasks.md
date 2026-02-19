## 1. CLI とスキーマ拡張

- [x] 1.1 `src/main.rs` に `run --wait` フラグを追加する（検証: `src/main.rs` に `--wait` の定義がある）
- [x] 1.2 `src/schema.rs` の `RunData` に `exit_code` / `finished_at` / `final_snapshot` を追加する（検証: `RunData` 定義で該当フィールドが確認できる）

## 2. 実行ロジック

- [x] 2.1 `src/run.rs` の `run` 実行経路に `--wait` の分岐を追加し、終端状態まで待機する（検証: `run` の実装で待機処理が `--wait` で有効になる）
- [x] 2.2 終了時点の `final_snapshot` を生成する処理を追加する（検証: `final_snapshot` 生成が `--wait` の経路で呼ばれる）

## 3. テスト

- [x] 3.1 `tests/integration.rs` に `run --wait` のシナリオを追加する（検証: 新しいテストケースが追加されている）
- [x] 3.2 `run --wait` のレスポンスに `finished_at` と `final_snapshot` が含まれることを検証する（検証: 該当アサーションが追加されている）

## Acceptance #1 Failure Follow-up

- [x] `run --wait` の `waited_ms` が終端状態までの待機時間を表していない。`src/run.rs` の `execute` で `waited_ms` を `snapshot_after` 待機時間から計算しており（L257-L285）、`--wait` の追加待機（L289-L303）が反映されないため、`--snapshot-after 0 --wait` でも `waited_ms=0` になる。`--wait` 時は終端までの実待機時間を返すよう修正する。（修正: `wait_start` タイマーを `snapshot_after` フェーズと `--wait` フェーズ両方にまたがる形で統合し、`waited_ms` が全待機時間を反映するよう変更）
- [x] `--wait` 指定時にも `snapshot_after` の 10,000ms クランプが適用されている。`src/run.rs` L251-L252 で常に `opts.snapshot_after.min(10000)` を使っており、spec の「`--wait` では上限 10,000ms を適用しない」要件を満たさない。`--wait` 時はクランプを無効化するか、`snapshot_after` を適用しない実装へ変更する。（修正: `--wait` 時は `effective_snapshot_after = 0` とし、`snapshot_after` フェーズを完全にスキップ。`final_snapshot` は `--wait` フェーズで取得）
