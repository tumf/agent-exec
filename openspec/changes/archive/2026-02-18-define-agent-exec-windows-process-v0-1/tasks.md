## 1. Windows プロセス管理

- [x] 1.1 Job Object を用いたプロセスツリー管理を実装する（検証: Windows 実行時に子プロセスが Job Object に割り当てられる）
- [x] 1.2 `kill` のシグナルマッピングを実装する（検証: `TERM`/`INT`/`KILL` が期待どおりに終了を誘発する）
- [x] 1.3 `state.json` に Job Object 識別情報を記録する（検証: Windows 実行時の `state.json` に識別子が含まれる）

## 実装メモ

### タスク 1.1: Job Object によるプロセスツリー管理
- `src/run.rs` の `supervise` 関数に `assign_to_job_object(job_id, pid)` を追加（Windows のみ）
- Job Object 名は `"AgentExec-{job_id}"` 形式の名前付き Job Object として作成
- 割り当て成功時は `state.json` の `windows_job_name` フィールドに名前を記録
- 割り当て失敗時（プロセスが既に別の Job Object に属する場合など）は `None` を返し、`kill` がスナップショット列挙フォールバックを使用

### タスク 1.2: kill シグナルマッピング
- `src/kill.rs` の `send_signal` 関数（Windows 版）を更新
- `TERM`/`INT`/`KILL` は全て Job Object 終了（`TerminateJobObject`）にマップ（設計メモ通り）
- 未知のシグナルも `KILL` 相当（`TerminateJobObject`）として処理
- `state.json` に `windows_job_name` が記録されていれば `OpenJobObjectW` で直接開いて終了
- 記録がなければ匿名 Job Object 割り当て経由でのフォールバック処理

### タスク 1.3: state.json への Job Object 識別情報記録
- `src/schema.rs` の `JobState` 構造体に `windows_job_name: Option<String>` フィールドを追加
- `#[serde(skip_serializing_if = "Option::is_none")]` により非 Windows では JSON に含まれない
- Windows で Job Object 割り当て成功時のみ値が設定される

## Acceptance #1 Failure Follow-up

- [x] `src/run.rs` の `supervise` で Job Object 割り当て失敗時に処理を継続しないように修正し、Windows の MUST 要件（子プロセスを Job Object に割り当てる）を満たす
- [x] `src/jobstore.rs` の `init_state` と `src/run.rs` の `supervise` 更新ロジックを見直し、Windows で実行中の `state.json` が常に Job Object 識別情報を含むようにする

## Acceptance #2 Failure Follow-up

- [x] `src/jobstore.rs` の `init_state` が `windows_job_name: None` を書き込み（`src/jobstore.rs:173-183`）、`run` がその直後に返る（`src/run.rs:89-109`）ため、Windows の実行中 `state.json` に Job Object 識別子が常時含まれる要件を満たしていない。初期 state に決定論的な Job 名を記録するか、識別子付き state が書かれるまで `run` 完了を遅延させる。
  - 修正: `init_state` を変更し、Windows では `"AgentExec-{job_id}"` を決定論的に `windows_job_name` に設定するよう実装。`run` から返った直後でも `state.json` に Job Object 識別子が含まれる。
- [x] Windows で Job Object 割り当て失敗時、`supervise` は `Err` で終了するが（`src/run.rs:162-168`）、その前に起動した子プロセス（`src/run.rs:145-153`）の停止と失敗 state 反映が行われない。割り当て失敗時に子プロセスを確実に終了し、`state.json` を `failed` 等に更新し、`run` 側でも失敗を検知できるハンドシェイクを実装する。
  - 修正: Job Object 割り当て失敗時に `child.kill()` + `child.wait()` で子プロセスを確実に終了し、`state.json` を `Failed` 状態に更新してから `Err` を返すよう実装。`run` 側でも Windows で最大 5 秒のポーリングハンドシェイクを追加し、`state.json` が `failed` になった場合に失敗を検知してエラーを返すよう実装。
