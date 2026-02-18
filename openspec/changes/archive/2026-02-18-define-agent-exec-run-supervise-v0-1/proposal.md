# 変更提案: agent-exec v0.1 run/監視・出力仕様

## 概要

`run` の実行監視、スナップショット、ログ取得、timeout、環境変数注入、`status/tail/wait/kill` の JSON 出力を固定する。

## 目的

- `run` が短命で終了してもジョブ監視とログ更新が継続することを保証する
- `snapshot`/`tail` が安定して取得できる形式と制約を決める

## スコープ

- `run` の監視分離とスナップショット挙動
- `tail-lines`/`max-bytes`/`snapshot-after`/`timeout`/`kill-after`/`cwd`/`env`/`env-file`/`inherit-env`/`mask`/`log`/`progress-every`
- `run`/`status`/`tail`/`wait`/`kill` の JSON 出力内容
- ログファイル取得方式（stdout.log/stderr.log の末尾読取）

## スコープ外

- Windows のプロセスツリー管理（別変更で扱う）
- ジョブ保存先の決定やディレクトリ構造（別変更で扱う）
