## ADDED Requirements

### Requirement: list の cwd フィルタ

`list` は `meta.json.cwd` が対象ディレクトリと一致するジョブのみを返さなければならない（MUST）。既定の対象ディレクトリは `list` 実行プロセスの current_dir とする（MUST）。`--cwd <PATH>` が指定された場合は、そのパスを対象ディレクトリとして扱わなければならない（MUST）。`--all` が指定された場合は cwd フィルタを無効化し、対象ディレクトリ一致の条件を適用してはならない（MUST）。対象ディレクトリと `meta.json.cwd` は同一の正規化規則（可能なら `canonicalize`、失敗時は絶対化）で比較しなければならない（MUST）。

#### Scenario: デフォルトの current_dir 一致
- **WHEN** current_dir が `A` の状態で `agent-exec list` を実行する
- **THEN** `jobs` は `meta.json.cwd == A` のジョブのみを含む

#### Scenario: --cwd 指定のフィルタ
- **WHEN** current_dir が `B` の状態で `agent-exec list --cwd A` を実行する
- **THEN** `jobs` は `meta.json.cwd == A` のジョブのみを含む

#### Scenario: --all による全件表示
- **WHEN** current_dir が `B` の状態で `agent-exec list --all` を実行する
- **THEN** `jobs` は cwd 一致条件で除外されない

### Requirement: list の --all/--cwd 排他

`list` は `--all` と `--cwd` の同時指定を受け付けてはならず、usage エラーとして終了しなければならない（MUST）。

#### Scenario: --all と --cwd の同時指定
- **WHEN** `agent-exec list --all --cwd /tmp` を実行する
- **THEN** コマンドは usage エラーとして終了コード 2 を返す
