## Why

agent-exec にはエージェント向けスキルの導入機構がなく、CLI 利用者が手作業で .agents 配下を整備する必要があります。slack-rs の install-skills と整合するインストール経路を追加し、非対話・JSON-only の契約を保ったままスキル配布を自動化するために本変更を行います。

## What Changes

- `install-skills` サブコマンドを追加し、`self`/`local:<path>` ソースと `--global` をサポートする
- 埋め込みスキル（`skills/agent-exec/**`）をバイナリへ取り込み、`.agents/skills/` へ展開する
- `.agents/.skill-lock.json` を作成/更新し、インストール結果を追跡できるようにする
- `type="install_skills"` の成功レスポンスとエラーコードを定義する
- プロジェクトローカルインストール向けの統合テストを追加する

## Capabilities

### New Capabilities
- `agent-exec-skills`: `install-skills` の入力解釈、展開、lock 更新、JSON レスポンス

### Modified Capabilities
- `agent-exec-contract`: CLI サブコマンド構成に `install-skills` を追加

## Impact

- 変更対象: `src/main.rs`, `src/lib.rs`, `src/schema.rs`, 新規 `src/skills.rs`/`src/install_skills.rs`, `tests/integration.rs`
- 追加リソース: `skills/agent-exec/**`
- 追加生成物: `.agents/skills/*`, `.agents/.skill-lock.json`
