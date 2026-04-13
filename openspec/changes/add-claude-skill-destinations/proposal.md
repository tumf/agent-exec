---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/skills.rs
  - src/install_skills.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-skills/spec.md
---

# Change Proposal: add-claude-skill-destinations

**Change Type**: implementation

## Problem/Context

`install-skills` は現在 `.agents/skills` と `~/.agents/skills` にのみインストールでき、Claude 系のローカル/グローバル skill ディレクトリへ直接配置できません。mini 上の運用では `~/.claude/skills` 配下を使うワークフローがあり、CLI から非対話で `.claude/skills` 系へ切り替えられる必要があります。

## Proposed Solution

- `install-skills` に `--claude` フラグを追加する
- `--claude` 未指定時は既存どおり `.agents/skills` / `~/.agents/skills` を使う
- `--claude` 指定時は `.claude/skills` / `~/.claude/skills` を使う
- lock file も選ばれたルート配下の `.skill-lock.json` に保存する
- 成功レスポンスの `skills[].path` と `lock_file_path` は実際の `.agents` / `.claude` ルートを反映する

## Acceptance Criteria

- `agent-exec install-skills --claude` は `<cwd>/.claude/skills/<skill_name>` にインストールする
- `agent-exec install-skills --claude --global` は `~/.claude/skills/<skill_name>` にインストールする
- `agent-exec install-skills` と `agent-exec install-skills --global` の既存 `.agents` 挙動は維持される
- `.skill-lock.json` は選択されたルート（`.agents` または `.claude`）配下で作成・更新される
- 既存の source 解釈 (`self`, `local:<path>`) と `unknown_source_scheme` エラー契約は維持される

## Out of Scope

- `install-skills` の source 種別追加
- 既存 `.agents` インストールの自動移行
- 他コマンドの `.claude` 対応
