## 1. run と監視

- [x] 1.1 `run` の監視分離と `snapshot-after` 返却を実装する（検証: `run` が JSON を返して終了し、監視が継続する）
- [x] 1.2 `stdout.log`/`stderr.log`/`full.log` の追記処理を実装する（検証: 実行後に各ログが更新される）

## 2. snapshot/tail と JSON 出力

- [x] 2.1 `tail-lines`/`max-bytes` に従う末尾取得を実装する（検証: `tail` が制約内の内容を返す）
- [x] 2.2 `run`/`status`/`tail`/`wait`/`kill` の JSON 出力を整備する（検証: stdout に必要フィールドが含まれる）

## 3. オプション挙動

- [x] 3.1 `timeout`/`kill-after` を実装する（検証: timeout 経過後にプロセスが終了する）
- [x] 3.2 `cwd`/`env`/`env-file`/`inherit-env`/`mask` の挙動を実装する（検証: 環境変数の適用順と mask 表示が確認できる）
- [x] 3.3 `--log` の保存先上書きを実装する（検証: 指定パスに `full.log` が作成される）
- [x] 3.4 `progress-every` による `state.json` 更新を実装する（検証: `updated_at` が周期的に更新される）

## Acceptance #1 Failure Follow-up

- [x] `--progress-every` のみ指定したジョブで子プロセス終了後に `_supervise` が停止しない問題を修正する（`src/run.rs` の watcher ループに終了条件を追加し、`status`/`wait` が `running` のまま残らないことを統合テストで確認する）。
- [x] CLI に `--inherit-env` オプションを追加し、`--no-inherit-env` との同時指定を clap の排他制約で拒否する（spec の「同時指定不可」要件を満たす）。
- [x] `--mask` を JSON 出力と表示用メタデータへ適用する実装を追加し、`--env SECRET=... --mask SECRET` で値が `***` にマスクされることを統合テストで検証する。
