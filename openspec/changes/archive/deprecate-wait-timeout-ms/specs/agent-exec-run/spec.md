## MODIFIED Requirements

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドの待機期限指定は `--until <ms>` を正規のオプションとして扱わなければならない（MUST）。この待機期限はジョブ実行時間の timeout ではなく、CLI が終端状態を待つ最大時間を表さなければならない（MUST）。

必要な後方互換期間中のみ、既存の `--timeout-ms` は legacy alias として受け付けてもよい（MAY）。ただし、その場合でもユーザー向けの正規ドキュメント・例・テスト経路では `--until` を用いなければならない（MUST）。

#### Scenario: wait --until is the canonical spelling

Given a running job created by `agent-exec run -- sh -c "sleep 10"`
When `agent-exec wait --until 100 <job_id>` is executed
Then the response state is `created` or `running`
And `exit_code` is absent
And the documented primary wait-deadline flag is `--until`

#### Scenario: legacy timeout-ms remains non-canonical when supported

Given an implementation that still accepts `agent-exec wait --timeout-ms 100 <job_id>` for backward compatibility
When user-facing help, README examples, and normative tests are reviewed
Then they use `--until` as the primary spelling
And `--timeout-ms` is identified as legacy or deprecated terminology
