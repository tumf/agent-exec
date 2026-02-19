# 変更提案: list の状態フィルタ追加

## 背景
`agent-exec list` は現在、全ジョブを返すため、実行中ジョブの監視用途ではクライアント側で追加のフィルタが必要です。

## 目的
`list` に状態フィルタを追加し、実行中など特定状態のジョブを簡単に取得できるようにします。

## スコープ
- `list` に `--state <state>` を追加する
- フィルタ適用後に `--limit` と `truncated` を評価する
- 統合テストにフィルタ動作の確認を追加する

## スコープ外
- `list` の JSON 形状変更
- 新しい状態種別の追加
- 既存ジョブ状態の算出ロジックの変更

## 変更概要
- 許容される `state` は `running|exited|killed|failed|unknown`
- `--state` 未指定時は現状どおり全件を返す
- `--state` 指定時は `jobs[].state` が一致するものだけ返す

## Why
`agent-exec list` は現在全ジョブを返すため、実行中ジョブの監視にはクライアント側フィルタが必要でした。サーバーサイドで状態フィルタを提供することで、監視ツールやスクリプトの実装を簡潔にできます。

## What Changes
- `list` サブコマンドに `--state <state>` オプションを追加する
- 許容される値は `running|exited|killed|failed|unknown`（未知の値は usage エラー）
- フィルタ適用後の件数に対して `--limit` を評価し、`truncated` フラグを設定する
- `agent-exec` スペックに「Requirement: list の状態フィルタ」要件を追加する
