## 依存クレートの追加

- [x] 1.1 `Cargo.toml` に `axum`・`tokio` を追加する（検証: `Cargo.toml` に `axum = "0.8"`, `tokio = { version = "1", features = ["full"] }` が含まれる）

## serve サブコマンドの実装

- [x] 2.1 `src/serve.rs` を新規作成し、axum ルーターと `ServeOpts` 構造体を実装する（検証: `src/serve.rs` が存在し `pub fn execute(opts: ServeOpts) -> Result<()>` が定義されている）
- [x] 2.2 `src/main.rs` に `Command::Serve { bind, port, root }` バリアントを追加し `serve::execute` を呼ぶ（検証: `src/main.rs` に `Serve` バリアントがあり `agent-exec serve --help` が動く）
- [x] 2.3 serve の既定アドレスを `127.0.0.1:18080` にする（検証: `agent-exec serve --help` に既定値表示が `127.0.0.1:18080`）
- [x] 2.3 `src/lib.rs` に `pub mod serve;` を追加する（検証: `src/lib.rs` に `pub mod serve;` が含まれる）

## エンドポイント実装

- [x] 3.1 `GET /health` を実装し `{"ok":true,"schema_version":"...","type":"health"}` を返す（検証: `curl -s http://127.0.0.1:18080/health` が `ok=true` を含む JSON を返す）
- [x] 3.2 `POST /exec` を実装し `run::execute` を呼び job_id を含む `RunData` を返す（検証: `curl -s -X POST ... /exec` のレスポンスに `job_id` が含まれる）
- [x] 3.3 `GET /status/:id` を実装し `status::execute` 相当の結果を返す（検証: `/status/<job_id>` が `state` フィールドを含む JSON を返す）
- [x] 3.4 `GET /tail/:id` を実装し `tail::execute` 相当の結果を返す（検証: `/tail/<job_id>` がログ末尾を含む JSON を返す）
- [x] 3.5 `GET /wait/:id` を実装し `wait::execute` 相当の結果を返す（検証: `/wait/<job_id>` がジョブ終端後に `state` が終端値の JSON を返す）
- [x] 3.6 `POST /kill/:id` を実装し `kill::execute` 相当の操作を行う（検証: ジョブ起動後 `/kill/<job_id>` が `ok=true` を返しジョブが停止する）

## エラーハンドリング

- [x] 4.1 job_id が存在しない場合は HTTP 404 + `{"ok":false,"error":{"code":"job_not_found",...}}` を返す（検証: `curl -s /status/nonexistent_id` がステータス 404 を返す）
- [x] 4.2 リクエストボディが不正な場合は HTTP 400 + `{"ok":false,"error":{"code":"invalid_request",...}}` を返す（検証: 空ボディで `POST /exec` するとステータス 400 が返る）
- [x] 4.3 内部エラーは HTTP 500 + `{"ok":false,"error":{"code":"internal_error",...}}` を返す（検証: エラーパス用の単体テストで 500 レスポンスを確認する）

## テスト

- [x] 5.1 `tests/serve_integration.rs` を新規作成し、`serve` を背景スレッドで起動して全エンドポイントを実際にリクエストするテストを書く（検証: `cargo test --test serve_integration` が全件パスする）
- [x] 5.2 既存の統合テストスイートが全件パスすることを確認する（検証: `cargo test` が全件パスする）

## ドキュメント

- [x] 6.1 `README.md` の使い方セクションに `agent-exec serve` の説明・エンドポイント一覧・注意事項（0.0.0.0 バインドのネットワーク制御必須）を追記する（検証: `README.md` に `serve` セクションが存在し Flowise 利用例が含まれる）

## Future Work

- OpenAPI ドキュメント自動生成
- Bearer トークン認証オプション
- WebSocket を使った stdout/stderr ストリーミング
