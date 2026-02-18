## 1. CLI 契約の反映

- [x] 1.1 サブコマンドと `run -- <cmd>` 形式の引数定義を追加する（検証: CLI 定義ファイルに run/status/tail/wait/kill が存在）
- [x] 1.2 ヘルプ文言を英語で定義する（検証: `--help` の文言が英語である）

## 2. 共通レスポンスと終了コード

- [x] 2.1 共通 JSON エンベロープと error オブジェクト型を追加する（検証: 型定義に `schema_version`, `ok`, `type`, `error` が存在）
- [x] 2.2 終了コードのマッピングを実装する（検証: 期待失敗が exit code 1 になることを確認）

## 3. stdout/stderr 分離

- [x] 3.1 stdout JSON-only と stderr ログ分離を保証する（検証: コマンド実行時 stdout が JSON のみになる）

## Acceptance #1 Failure Follow-up

- [x] CLI 実行名を仕様どおり `agent-exec` で呼び出せるようにする（`Cargo.toml` に `[[bin]] name = "agent-exec"` を追加し、`tests/integration.rs` の `binary()` 関数も `agent-exec` を参照するよう修正。全18テスト通過確認）。
