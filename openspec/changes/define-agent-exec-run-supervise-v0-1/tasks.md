## 1. run と監視

- [ ] 1.1 `run` の監視分離と `snapshot-after` 返却を実装する（検証: `run` が JSON を返して終了し、監視が継続する）
- [ ] 1.2 `stdout.log`/`stderr.log`/`full.log` の追記処理を実装する（検証: 実行後に各ログが更新される）

## 2. snapshot/tail と JSON 出力

- [ ] 2.1 `tail-lines`/`max-bytes` に従う末尾取得を実装する（検証: `tail` が制約内の内容を返す）
- [ ] 2.2 `run`/`status`/`tail`/`wait`/`kill` の JSON 出力を整備する（検証: stdout に必要フィールドが含まれる）

## 3. オプション挙動

- [ ] 3.1 `timeout`/`kill-after` を実装する（検証: timeout 経過後にプロセスが終了する）
- [ ] 3.2 `cwd`/`env`/`env-file`/`inherit-env`/`mask` の挙動を実装する（検証: 環境変数の適用順と mask 表示が確認できる）
- [ ] 3.3 `--log` の保存先上書きを実装する（検証: 指定パスに `full.log` が作成される）
- [ ] 3.4 `progress-every` による `state.json` 更新を実装する（検証: `updated_at` が周期的に更新される）
