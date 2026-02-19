# agent-exec-skills Specification

## ADDED Requirements

### Requirement: インストールソースの解釈

`install-skills` は `self`（省略時含む）と `local:<path>` のみを受け付けなければならない（MUST）。未知のスキームは `error.code="unknown_source_scheme"` で失敗しなければならない（MUST）。

#### Scenario: 未知スキームの拒否
Given `agent-exec install-skills github:user/repo` を実行する
When コマンドが完了する
Then stdout は `ok=false` の JSON を返し `error.code="unknown_source_scheme"` を含む

### Requirement: インストール先の解決

`--global` が未指定の場合、スキルは `<cwd>/.agents/skills/<skill_name>` に展開されなければならない（MUST）。`--global` 指定時は `~/.agents/skills/<skill_name>` に展開されなければならない（MUST）。

#### Scenario: ローカルインストールのパス
Given 空の作業ディレクトリで `agent-exec install-skills` を実行する
When コマンドが完了する
Then `skills[0].path` は `<cwd>/.agents/skills/agent-exec` 配下の絶対パスである

### Requirement: 埋め込みスキルの展開

`self` ソースは埋め込みスキルを展開しなければならない（MUST）。展開先には `SKILL.md` が存在しなければならない（MUST）。

#### Scenario: SKILL.md の配置
Given `agent-exec install-skills` を実行する
When コマンドが完了する
Then `<cwd>/.agents/skills/agent-exec/SKILL.md` が存在する

### Requirement: lock ファイルの更新

`install-skills` は `.agents/.skill-lock.json` を作成または更新し、インストール済みスキルを記録しなければならない（MUST）。記録には `name` と `path` と `source_type` を含めなければならない（MUST）。

#### Scenario: lock への記録
Given `agent-exec install-skills` を実行する
When コマンドが完了する
Then `.agents/.skill-lock.json` に `name="agent-exec"` のエントリが含まれる

### Requirement: 成功レスポンスの構造

成功時の JSON は `type="install_skills"` を含み、`skills` 配列と `global` と `lock_file_path` を返さなければならない（MUST）。`skills[]` の各要素は `name`/`path`/`source_type` を含まなければならない（MUST）。

#### Scenario: JSON ペイロード
Given `agent-exec install-skills` を実行する
When コマンドが完了する
Then JSON に `skills` と `global` と `lock_file_path` が含まれる
