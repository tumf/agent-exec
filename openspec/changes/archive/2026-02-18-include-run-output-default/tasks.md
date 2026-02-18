## 1. 仕様更新

- [x] 1.1 変更仕様を反映した spec を作成する（検証: `openspec/changes/include-run-output-default/specs/agent-exec-run/spec.md` に要件と Scenario が存在する）

## 2. run の既定スナップショット

- [x] 2.1 `run` の `snapshot_after` デフォルト値を 200ms に変更する（検証: `src/main.rs` の `snapshot_after` デフォルト値が 200、`agent-exec run --help` で表示される）

## 3. スナップショット待機のポーリング化

- [x] 3.1 `run` の待機ロジックをポーリングに変更し、出力または状態変化で早期終了できるようにする（検証: `src/run.rs` の待機ロジックがポーリング実装に置換されている）

## 4. stdout/stderr のバイト取得

- [x] 4.1 監視プロセスで stdout/stderr をバイト単位で取得し、`stdout.log`/`stderr.log` にそのまま追記する（検証: `src/run.rs` の監視処理が `Read::read` ベースになっている）
- [x] 4.2 `full.log` の行フォーマットを維持するため、未完了行バッファを用いた整形を追加する（検証: `src/run.rs` の `full.log` 書き込みが行バッファを用いる実装になっている）

## 5. 統合テスト更新

- [x] 5.1 既定 `run` で `snapshot` が返ることを検証する統合テストを追加・更新する（検証: `tests/integration.rs` に新規テストがあり、`cargo test --test integration run_default_includes_snapshot` が通る）
- [x] 5.2 改行なし出力が `snapshot` に含まれることを検証するテストを追加する（検証: `tests/integration.rs` に該当テストがあり、`cargo test --test integration run_snapshot_captures_output_without_newline` が通る）
