# 設計メモ: run の既定スナップショットと出力取得

## 方針

1. `run` の既定 `snapshot_after` を短時間（例: 200ms）に変更し、デフォルトで `snapshot` を返す。
2. `snapshot_after` の待機は固定 sleep ではなく、出力の有無やジョブ状態をポーリングして早期終了できるようにする。
3. 監視プロセスでの stdout/stderr 取り込みは「行単位」ではなく「バイト単位」に変更し、改行なし出力でも `stdout.log`/`stderr.log` に反映されるようにする。
4. `full.log` の既存フォーマット（`<RFC3339> [STDOUT] <line>`）は維持する。

## 具体案

### 1) デフォルト `snapshot_after` の変更

- CLI の `snapshot_after` デフォルト値を 0 → 200 に変更
- `run` の JSON には `snapshot` が既定で含まれるようになる

### 2) スナップショット待機のポーリング化

- `deadline = now + clamp(snapshot_after, 0..=10_000ms)` を設定
- 10〜25ms 間隔で以下を確認し、条件を満たせば即時に `snapshot` 生成:
  - `stdout.log` または `stderr.log` のサイズが 1 byte 以上
  - `state.json` が `running` 以外になった
  - `deadline` 到達
- `waited_ms` は実際の待機時間を返す

### 3) 監視プロセスのストリーム取得方式

- `BufRead::lines()` ではなく `Read::read()` でチャンク読み
- `stdout.log` / `stderr.log` は読み取ったバイト列をそのまま追記
- `full.log` は既存の行ベース仕様を維持するため、ストリーム毎に「未完了行バッファ」を持ち、改行で分割して行として出力する
- EOF 時に残バッファがあれば 1 行として出力

## トレードオフ

- デフォルト待機時間の導入により、`run` の応答がわずかに遅くなる
- `run` の JSON サイズが増える
- 既存ログフォーマットとの互換性維持のため、`full.log` では行バッファ処理が必要

## 影響範囲

- CLI のデフォルト値変更（`src/main.rs`）
- `run` のスナップショット待機ロジック（`src/run.rs`）
- 監視プロセスのログ収集（`src/run.rs`）
- 統合テストの更新・追加（`tests/integration.rs`）
