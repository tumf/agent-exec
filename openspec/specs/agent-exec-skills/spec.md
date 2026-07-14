# agent-exec-skills Specification

## Requirements

### Requirement: 埋め込みスキル専用インストール

`install-skills` は埋め込みの `agent-exec` スキルだけをインストールしなければならない（MUST）。外部ソースや任意のローカルスキルを指定する入力は受け付けてはならない（MUST NOT）。

### Requirement: インストール先の解決

`--global` が未指定の場合、スキルは `<cwd>/.agents/skills/<skill_name>` に展開されなければならない（MUST）。`--global` 指定時は `~/.agents/skills/<skill_name>` に展開されなければならない（MUST）。

#### Scenario: ローカルインストールのパス
Given 空の作業ディレクトリで `agent-exec install-skills` を実行する
When コマンドが完了する
Then `skills[0].path` は `<cwd>/.agents/skills/agent-exec` 配下の絶対パスである

### Requirement: 埋め込みスキルの展開

`install-skills` は埋め込みの `agent-exec` スキルを展開しなければならない（MUST）。展開先には `SKILL.md` が存在しなければならない（MUST）。

#### Scenario: SKILL.md の配置
Given `agent-exec install-skills` を実行する
When コマンドが完了する
Then `<cwd>/.agents/skills/agent-exec/SKILL.md` が存在する

### Requirement: lock ファイルの更新

`install-skills` は `.agents/.skill-lock.json` を作成または更新し、インストール済みスキルを記録しなければならない（MUST）。記録には `name` と `path` と `source_type` を含めなければならない（MUST）。`source_type` は埋め込みインストールであることを示さなければならない（MUST）。

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

### Requirement: MCP-first managed job guidance

Embedded `skills/agent-exec/SKILL.md` must document MCP-first operation for MCP-capable agent clients such as Hermes (MUST). The skill must instruct such clients to start uncertain-duration, long-running, or high-output commands through agent-exec MCP `run` rather than a terminal process manager (MUST), retain returned job IDs, and observe jobs with MCP `status`, `tail`, or bounded `wait` (MUST).

The skill must state that MCP `kill` is reserved for an explicit user cancellation request (MUST). MCP client disconnect, bounded wait expiry, missing output, or a transition to other work must not be treated as cancellation authorization (MUST NOT).

#### Scenario: installed skill documents bounded MCP lifecycle

**Given**: `agent-exec install-skills` installs the embedded skill
**When**: the installed `SKILL.md` is read
**Then**: it describes MCP `run`, job-ID retention, `status`/`tail`/bounded `wait`, and explicit-user-request-only `kill`

### Requirement: MCP fallback and Hermes configuration guidance

The embedded skill must include an MCP configuration example for Hermes Native MCP that launches `agent-exec mcp` (MUST). The skill must document CLI `agent-exec run -- <command>` as fallback when MCP is unavailable (MUST). It must preserve the exception for clearly short, synchronous, safe inline shell commands (MAY).

#### Scenario: installed skill retains CLI fallback

**Given**: `agent-exec install-skills` installs the embedded skill
**When**: the installed `SKILL.md` is read
**Then**: it includes a Hermes Native MCP configuration example
**And**: it includes CLI fallback guidance for MCP-unconfigured environments
**And**: it does not instruct a client to kill a job merely because an observation deadline elapsed
