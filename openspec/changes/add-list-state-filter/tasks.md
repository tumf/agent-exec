## 1. CLI 追加

- [x] 1.1 `list` に `--state <state>` オプションを追加する（clap の value_parser で許容値を限定する）
      Verify: `src/main.rs` の `Command::List` に `state` フィールドと `value_parser` がある
- [x] 1.2 `ListOpts` に `state` を追加して CLI から伝搬する
      Verify: `src/list.rs` の `ListOpts` と `agent_exec::list::execute` 呼び出しで `state` が渡っている

## 2. list フィルタ実装

- [x] 2.1 `jobs` 生成後に `state` フィルタを適用し、フィルタ後に `--limit` を評価する
      Verify: `src/list.rs` に `state` が一致する要素だけを残す処理があり、`limit` 適用がその後にある

## 3. 統合テスト

- [x] 3.1 `list --state running` の統合テストを追加する（長時間ジョブは `kill` で終了）
      Verify: `tests/integration.rs` にテストが追加され、`cargo test --test integration list_filters_by_state_running` が通る
