## 1. README の実例更新

- [ ] 1.1 `README.md` のコマンド例を `run/status/tail/wait/kill/list` に置き換える（検証: `README.md` に各サブコマンドの例がある）
- [ ] 1.2 JSON-only stdout と stderr ログの説明を追加する（検証: `README.md` に契約の説明がある）

## 2. 代表フローの追加

- [ ] 2.1 短命ジョブの `run --wait` 例を追加する（検証: `README.md` に該当コマンドがある）
- [ ] 2.2 長命ジョブの `run` → `status` → `tail` 例を追加する（検証: 連続手順の例がある）
- [ ] 2.3 タイムアウト/強制終了の例を追加する（検証: `--timeout`/`--kill-after` の例がある）
