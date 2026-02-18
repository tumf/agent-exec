# 変更提案: agent-exec v0.1 CLI/JSON 仕様固定

## 概要

agent-exec の v0.1 として CLI UX、JSON 出力スキーマ、デフォルト値を固定し、実装にそのまま落とせる仕様一式を定める。

## 目的

- エージェント/自動化が安定して利用できる非対話 CLI の仕様を確定する
- stdout を JSON のみに固定し、巨大出力はログファイルに退避する
- Windows を含むクロスプラットフォーム対応方針を明確化する

## スコープ

- run/status/tail/wait/kill の仕様固定
- JSON スキーマ（トップレベル共通、各コマンド出力）固定
- ログ/ジョブディレクトリ構造とデフォルトパス固定
- Windows 対応方針（プロセス管理、パス解決）固定

## スコープ外

- policy engine
- gRPC API
- NDJSON streaming
- secret store integration
- seccomp/sandbox

## 決定事項（ユーザー選択）

1. デフォルトのジョブ保存先: XDG data（`$XDG_DATA_HOME/agent-exec/jobs`）を優先し、未設定時は `~/.local/share/agent-exec/jobs`
2. Windows も v0.1 でサポート対象
3. tail/snapshot の stdout/stderr 文字列は UTF-8 lossy（`encoding="utf-8-lossy"`）

## 変更提案の分割

この変更は CLI 仕様・出力スキーマ・実行管理の整合が強く結びついているため、1 件の変更としてまとめる。
