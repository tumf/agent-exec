# 設計メモ: run/監視分離

## 背景

`run` が `snapshot-after` で終了した後も子プロセスを継続させるため、stdout/stderr の読み取りと state 更新を `run` 本体から切り離す必要がある。

## 方針

- `run` はフロントとして JSON を返し、監視は別プロセス（同一バイナリの内部サブコマンドなど）へ委譲する。
- `tail`/`snapshot` は `stdout.log`/`stderr.log` の末尾を読み取って生成する。
- `progress-every` は stdout には出力せず、`state.json` の更新間隔として扱う。
