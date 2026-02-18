# agent-exec run ログストリーミング互換

## ADDED Requirements

### Requirement: stdout/stderr ログ内容の互換

`run` が生成する `stdout.log` と `stderr.log` は、子プロセスの出力バイト列を順序どおりに保存しなければならない（MUST）。リファクタにより内容・順序・欠落が変化してはならない（MUST）。

#### Scenario: 連続出力の保存
Given `agent-exec run --snapshot-after 0 -- <cmd>` を実行し、`<cmd>` が stdout と stderr にそれぞれ複数行出力する
When コマンドが `run` の JSON を返す
Then `stdout.log` と `stderr.log` には出力と同じ順序・内容が保存される

### Requirement: full.log の行フォーマット互換

`full.log` の行は `<RFC3339> [STDOUT] <line>` または `<RFC3339> [STDERR] <line>` の形式で記録されなければならない（MUST）。リファクタによりこの形式が変わってはならない（MUST）。

#### Scenario: full.log の行形式
Given `agent-exec run --snapshot-after 0 -- <cmd>` を実行し、`<cmd>` が stdout と stderr に 1 行ずつ出力する
When `full.log` を読む
Then 各行が `RFC3339` 形式のタイムスタンプと `[STDOUT]` / `[STDERR]` プレフィックスを含む
