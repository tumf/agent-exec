# 設計メモ: Windows プロセスツリー

## 方針

- Windows では Job Object を使用してプロセスツリーを管理する。
- `run` は子プロセスを Job Object に割り当て、`kill` は Job Object の終了でツリーを停止する。
- `--signal` は `TERM|INT|KILL` を受け付け、Windows の API で可能な範囲でマップする。未対応は `KILL` 相当とする。
