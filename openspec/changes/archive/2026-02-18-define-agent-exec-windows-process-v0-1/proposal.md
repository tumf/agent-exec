# 変更提案: agent-exec v0.1 Windows プロセス管理

## 概要

Windows 環境でのプロセスツリー管理と `kill` のシグナル対応を固定する。

## 目的

- Windows で `kill` が確実にプロセスツリーを終了できるようにする
- シグナル互換性の振る舞いを明確化する

## スコープ

- Job Object 等を用いたプロセスツリー管理
- Windows での `kill` のシグナルマッピング
- Windows での `state.json` へのプロセス管理情報反映

## スコープ外

- CLI 契約や JSON 共通仕様
- ジョブ保存先の解決
