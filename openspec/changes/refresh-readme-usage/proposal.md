# 変更提案: refresh-readme-usage

## 背景/課題
`README.md` が実装と乖離しており、`run/status/tail/wait/kill/list` の実用的な導線が提示されていない。初見ユーザーの採用初速が落ちる。

## 目的
- README を実態に合わせて更新し、初回成功率を上げる
- JSON-only stdout の契約とログの使い方を明確化する

## スコープ
- README のコマンド例を現行 CLI に合わせて差し替え
- 代表的な3フロー（短命/長命/タイムアウト）を追加

## 非スコープ
- 仕様や挙動そのものの変更
- docs サイトの新設

## 成功指標
- README だけで `run` → `wait` → `tail` の流れを再現できる
- 使い方に関する問い合わせが減る
