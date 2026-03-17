## MODIFIED Requirements

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。
`create`/`start` ライフサイクルでは、`--env` の `KEY=VALUE` は永続化され、`start` 実行時に適用されなければならない（MUST）。`--env-file` はファイルパス参照として永続化され、`start` 実行時にそのパス群を指定順で読み込まなければならない（MUST）。

#### Scenario: create した env が start に反映される

Given `agent-exec create --env FOO=bar -- sh -c 'printf %s "$FOO"'` を実行する
When 後続で `agent-exec start <job_id> --wait` を実行する
Then 終了時のログ末尾に `bar` が含まれる

#### Scenario: env-file は start 時に読み込まれる

Given `agent-exec create --env-file ./job.env -- sh -c 'printf %s "$FOO"'` を実行する
And `job.env` の内容が `create` 後 `start` 前に更新される
When `agent-exec start <job_id> --wait` を実行する
Then 実行時の環境は更新後の `job.env` の内容を反映する

### Requirement: run の同期待機オプション

`run` は `--wait` が指定された場合、ジョブが終端状態 (`exited|killed|failed`) になるまで待機しなければならない（MUST）。`--wait` 指定時、`snapshot-after` の待機上限 (10,000ms) を適用してはならない（MUST）。
`--wait` 指定時の `run` JSON は `exit_code`（存在する場合）と `finished_at` を含めなければならない（MUST）。
`--wait` 指定時の `run` JSON は終了時点のログ末尾を示す `final_snapshot` を含めなければならない（MUST）。`final_snapshot` の構造と制約は既存の `snapshot` と同一でなければならない（MUST）。`--wait` 指定時の `waited_ms` は終端状態までの待機時間を示さなければならない（MUST）。
`start` が導入された後も、`wait` 相当の観測オプションは `start` に対して同等に機能しなければならない（MUST）。

#### Scenario: start でも終了まで待機する

Given `created` 状態のジョブが `sh -c "echo hi"` を実行するよう保存されている
When `agent-exec start <job_id> --wait` を実行する
Then `state` は `exited` である
And `final_snapshot.stdout_tail` に `hi` が含まれる
And `finished_at` が含まれる
