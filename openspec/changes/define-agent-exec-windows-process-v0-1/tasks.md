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
