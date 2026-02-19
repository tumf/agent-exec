## 1. README の実例更新

- [x] 1.1 `README.md` のコマンド例を `run/status/tail/wait/kill/list` に置き換える（検証: `README.md` に各サブコマンドの例がある）
- [x] 1.2 JSON-only stdout と stderr ログの説明を追加する（検証: `README.md` に契約の説明がある）

## 2. 代表フローの追加

- [x] 2.1 短命ジョブの `run --wait` 例を追加する（検証: `README.md` に該当コマンドがある）
- [x] 2.2 長命ジョブの `run` → `status` → `tail` 例を追加する（検証: 連続手順の例がある）
- [x] 2.3 タイムアウト/強制終了の例を追加する（検証: `--timeout`/`--kill-after` の例がある）

## Acceptance #1 Failure Follow-up

- [x] `README.md:25` の `agent-exec run --wait ...` は CLI 実装 `src/main.rs:25`（`enum Command::Run`）に `--wait` フラグが存在せず実行不可（`cargo run --bin agent-exec -- run --wait echo "hello"` で `unexpected argument '--wait'`）。実装に存在する手順（例: `run` + `wait`）に修正し、README の全コマンド例を実コマンドで再検証する。
  - 修正: `Short-lived job` セクションを `run` → `wait` → `tail` の3ステップフローに変更し、全コマンド例を実際のバイナリで再検証済み。
