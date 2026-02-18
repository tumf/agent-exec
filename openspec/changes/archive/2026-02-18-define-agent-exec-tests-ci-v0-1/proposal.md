# 変更提案: agent-exec v0.1 テストと CI

## 概要

run/status/tail/wait/kill の JSON 出力を検証する統合テストと、Windows を含む CI マトリクスを固定する。

## 目的

- JSON-only stdout の契約を自動検証する
- Windows 実行の回帰を早期に検出する

## スコープ

- CLI の統合テスト（JSON スキーマ/終了コード）
- CI マトリクスに `windows-latest` を追加

## スコープ外

- 実装詳細の最適化
