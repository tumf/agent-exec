# 変更提案: job データのガベージコレクションを追加する

## Problem/Context
- `agent-exec` は job ごとに `<root>/<job_id>/` 配下へ `meta.json`, `state.json`, `stdout.log`, `stderr.log`, `full.log` などを永続化します。
- 現状の CLI には古い job ディレクトリを削除する仕組みがなく、長期運用時に root 配下のディスク使用量が単調増加します。
- 既存の `list` は cwd フィルタ、`status` / `tail` / `wait` / `kill` は個別 job 参照を担っており、自動削除を既存コマンドへ混ぜると予期しない副作用を増やします。
- 今回の前提は、GC は明示コマンドとして提供し、保持条件は終了後の経過時間ベースにし、既定は実削除で `--dry-run` により事前確認できるようにすることです。

## Proposed Solution
- 新しい `agent-exec gc` サブコマンドを追加し、root 配下の job ディレクトリを走査して削除候補を判定します。
- GC は terminal state の job (`exited|killed|failed`) のみを対象にし、`running` は常に保持します。
- 保持条件は既定で 30 日とし、`--older-than <duration>` を省略した場合は `30d` 相当として扱います。基準時刻は `state.json.finished_at` を優先し、欠落時のみ `updated_at` を使用します。両方欠落または state 解析不能の job は安全側でスキップします。
- `gc` は既定で削除を実行し、`--dry-run` 指定時は候補と見積りのみを返します。
- レスポンスは既存方針に合わせて JSON-only とし、削除・スキップの内訳と回収バイト数を返します。

## Acceptance Criteria
- `agent-exec gc` が既定の 30 日保持で terminal state の古い job ディレクトリだけを再帰削除する。
- `agent-exec gc --older-than <duration>` が既定値を上書きして terminal state の古い job ディレクトリだけを再帰削除する。
- `running` job と retention 判定に必要な情報が欠ける job は削除されず、JSON レスポンス上でスキップとして観測できる。
- `agent-exec gc --dry-run --older-than <duration>` は実際には削除せず、削除対象と回収予定バイト数を返す。
- 削除後の job は `status` / `tail` / `wait` / `kill` で既存どおり `job_not_found` として扱われる。
- README と OpenSpec が `gc` の使い方と安全性を説明する。

## Out of Scope
- `run`, `list`, またはバックグラウンド処理に紐づく自動 GC
- 件数上限ベースの保持ポリシー
- cwd ベースの削除対象制限
- 圧縮アーカイブや部分ファイル削除など、job ディレクトリを残す保持方式
