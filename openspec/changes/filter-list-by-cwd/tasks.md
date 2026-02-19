## 1. CLI とスキーマ更新

- [x] 1.1 `src/main.rs` の `list` に `--cwd <PATH>` と `--all` を追加し、`--all` と `--cwd` を排他にする（検証: `Command::List` の引数定義に `cwd`/`all` と排他設定がある）
- [x] 1.2 `src/schema.rs` の `JobMeta` に `cwd: Option<String>` を追加する（検証: `JobMeta` 構造体で `cwd` が定義されている）

## 2. run 側の cwd 保存

- [x] 2.1 `src/run.rs` で実効 cwd を解決する処理を追加し、`meta.json` に保存する（検証: `JobMeta { cwd: ... }` の代入と正規化処理がある）
- [x] 2.2 cwd 正規化のフォールバック（`canonicalize` 失敗時の絶対化）を実装する（検証: 正規化関数で `canonicalize` とフォールバックの分岐が確認できる）

## 3. list の cwd フィルタ

- [x] 3.1 `src/list.rs` で対象 cwd を決定する（`--all` 無しなら `--cwd` または current_dir を採用）処理を追加する（検証: `--all`/`--cwd`/current_dir の優先順位がコードに反映されている）
- [x] 3.2 `meta.json.cwd` と対象 cwd の一致で絞り込み、`--all` 時はフィルタを無効化する（検証: `jobs` への追加前に cwd 一致条件が適用される）

## 4. テスト更新

- [x] 4.1 `tests/integration.rs` に current_dir を切り替えて `list` の既定フィルタを検証するテストを追加する（検証: 新規テストで `Command::current_dir(...)` を使用している）
- [x] 4.2 `list --cwd` と `list --all` の挙動を検証するテストを追加する（検証: 該当オプションのテストケースが追加されている）
- [x] 4.3 `list --all --cwd` の usage エラー（終了コード 2）を検証するテストを追加する（検証: 失敗ケースの終了コードアサーションが追加されている）
