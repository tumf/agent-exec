## MODIFIED Requirements

### Requirement: ジョブ保存先の優先順位

ジョブ保存先は top-level の `--root` -> `AGENT_EXEC_ROOT` -> `$XDG_DATA_HOME/agent-exec/jobs` -> 既定パスの順に解決しなければならない（MUST）。既定パスは Unix 系では `~/.local/share/agent-exec/jobs`、Windows では `BaseDirs::data_local_dir()/agent-exec/jobs` としなければならない（MUST）。`--root` は job store を扱う全サブコマンドで一貫して使えるグローバルオプションとして提供しなければならない（MUST）。

#### Scenario: グローバル --root の適用

Given `AGENT_EXEC_ROOT` と `XDG_DATA_HOME` が未設定である
When `agent-exec --root /tmp/custom-jobs status <job_id>` を実行する
Then `/tmp/custom-jobs` 配下から対象ジョブが探索される

#### Scenario: グローバル --root が環境変数より優先される

Given `AGENT_EXEC_ROOT=/tmp/from-env` が設定されている
When `agent-exec --root /tmp/from-flag list --all` を実行する
Then `/tmp/from-flag` が job 保存先として使われる

### Requirement: CLI の root 構文一貫性

`agent-exec` は job store を扱うサブコマンドに対して、`agent-exec --root <PATH> <subcommand> ...` の構文を一貫して提供しなければならない（MUST）。CLI ヘルプと README の使用例はこの正規化された構文を優先して示さなければならない（MUST）。既存の per-subcommand `--root` 構文を移行期間中に許容する場合、その扱いは明示的でテスト可能でなければならない（MUST）。

#### Scenario: README がグローバル構文を示す

Given リポジトリの `README.md` を読む
When root 指定付きの使用例を確認する
Then `agent-exec --root <PATH> <subcommand> ...` の形式が使われている

#### Scenario: 旧構文の移行挙動が固定される

Given 旧来の per-subcommand `--root` 構文に対する移行方針が実装されている
When `tests/integration.rs` の CLI 互換性テストを実行する
Then 旧構文の受理または usage エラーの挙動が仕様どおりに固定される
