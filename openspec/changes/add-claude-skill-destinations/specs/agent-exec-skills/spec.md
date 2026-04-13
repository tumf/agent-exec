## MODIFIED Requirements

### Requirement: インストール先の解決

`install-skills` は既定で `.agents` ルートにインストールしなければならない（MUST）。`--global` 指定時はホームディレクトリ配下の対象ルートを使わなければならない（MUST）。`--claude` 指定時は `.claude` ルートへ切り替え、未指定時は `.agents` ルートを維持しなければならない（MUST）。

#### Scenario: ローカル install は既定で .agents を使う
Given 空の作業ディレクトリで `agent-exec install-skills` を実行する
When コマンドが完了する
Then `skills[0].path` は `<cwd>/.agents/skills/agent-exec` 配下の絶対パスである

#### Scenario: ローカル install で --claude は .claude を使う
Given 空の作業ディレクトリで `agent-exec install-skills --claude` を実行する
When コマンドが完了する
Then `skills[0].path` は `<cwd>/.claude/skills/agent-exec` 配下の絶対パスである

#### Scenario: グローバル install で --claude はホーム配下 .claude を使う
Given 空の作業ディレクトリで `agent-exec install-skills --claude --global` を実行する
When コマンドが完了する
Then `skills[0].path` は `~/.claude/skills/agent-exec` 配下の絶対パスである

### Requirement: lock ファイルの更新

`install-skills` は選択されたルート配下の `.skill-lock.json` を作成または更新し、インストール済みスキルを記録しなければならない（MUST）。記録には `name` と `path` と `source_type` を含めなければならない（MUST）。

#### Scenario: --claude install は .claude 配下 lock を更新する
Given `agent-exec install-skills --claude` を実行する
When コマンドが完了する
Then `<cwd>/.claude/.skill-lock.json` に `name="agent-exec"` のエントリが含まれる

### Requirement: 成功レスポンスの構造

成功時の JSON は `type="install_skills"` を含み、`skills` 配列と `global` と `lock_file_path` を返さなければならない（MUST）。`skills[]` の各要素は `name`/`path`/`source_type` を含まなければならない（MUST）。`lock_file_path` は選択されたルート配下の `.skill-lock.json` を指さなければならない（MUST）。

#### Scenario: --claude install は .claude 配下 lock path を返す
Given `agent-exec install-skills --claude` を実行する
When コマンドが完了する
Then JSON の `lock_file_path` は `<cwd>/.claude/.skill-lock.json` の絶対パスである
