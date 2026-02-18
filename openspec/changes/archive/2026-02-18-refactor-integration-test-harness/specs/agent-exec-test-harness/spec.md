# agent-exec 統合テストハーネス互換

## ADDED Requirements

### Requirement: テスト用ルート設定の一貫性

統合テストは一時ディレクトリを作成し、`AGENT_EXEC_ROOT` をそのパスに設定したうえでコマンドを実行しなければならない（MUST）。ハーネス化してもこの前提は維持されなければならない（MUST）。

#### Scenario: ルート指定の反映
Given 統合テストで一時ディレクトリを生成する
When テストハーネス経由で `agent-exec run --snapshot-after 0 -- <cmd>` を実行する
Then `meta.json`/`state.json` は一時ディレクトリ配下に作成される
