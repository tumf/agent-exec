# 変更提案: add-serve-subcommand

## 背景/課題

agent-exec は非対話型ジョブランナーとして CLI のみを提供している。
Flowise などの外部サービスやコンテナ化されたワークフローエンジンからジョブを起動・監視するには、
現状では CLI を直接呼び出せる実行環境が必要となり、サービスメッシュ内での利用に制約がある。
HTTP インターフェースがあれば、Flowise の「HTTP Request」ノードから直接 `POST /exec` してジョブを起動でき、
`GET /status/:id` でポーリングするだけの単純なフローで統合できる。

## 目的

- `agent-exec serve` サブコマンドで REST API サーバを起動できるようにする
- 既存の CLI サブコマンドと同等の操作を HTTP 経由で提供する
- Flowise／curl／任意の HTTP クライアントから認証なしで利用可能にする

## スコープ

- `agent-exec serve` サブコマンドの追加
- エンドポイント: `POST /exec`, `GET /status/:id`, `GET /tail/:id`, `POST /kill/:id`, `GET /wait/:id`, `GET /health`
- バインドアドレスのデフォルトは `127.0.0.1:18080`（`--bind` で上書き可）
- ポートのみ変更する場合は `--port` でも指定可
- 認証なし（ローカルホスト限定バインドによるアクセス制御を前提）
- JSON レスポンスは既存の CLI スキーマ（`schema_version`, `ok`, `type`, ...）に準拠

## 非スコープ

- TLS 終端（Caddy などリバースプロキシ層に委譲）
- 認証・認可
- WebSocket ストリーミング
- OpenAPI ドキュメント生成

## エンドポイント一覧

| Method | Path              | 対応 CLI               | 説明                                 |
|--------|-------------------|------------------------|--------------------------------------|
| GET    | /health           | ―                      | サーバ死活確認。`{"ok":true}` を返す |
| POST   | /exec             | `run`                  | ジョブを起動し job_id を返す         |
| GET    | /status/:id       | `status`               | ジョブ状態を返す                     |
| GET    | /tail/:id         | `tail`                 | stdout/stderr 末尾を返す             |
| GET    | /wait/:id         | `wait`                 | 終端状態まで待機して返す             |
| POST   | /kill/:id         | `kill`                 | ジョブにシグナルを送る               |

## POST /exec リクエストボディ例

```json
{
  "command": ["bash", "-c", "echo hello"],
  "cwd": "/tmp",
  "env": {"FOO": "bar"},
  "timeout_ms": 30000,
  "wait": false
}
```

## 成功指標

- `agent-exec serve` が起動し、既定で `127.0.0.1:18080` で HTTP リクエストを受け付ける。`--bind 0.0.0.0` 指定時のみ外部公開可。
- Flowise コンテナから `host.docker.internal:18080/exec` に POST してジョブが起動できる
- `GET /health` が `{"ok":true}` を返す
- 既存 CLI サブコマンドのテストが全件パスし続ける

## 依存/リスク

- axum + tokio を Cargo.toml に追加（ビルドサイズ増加、初回ビルド時間増加）
- デフォルト `127.0.0.1` バインドのため外部からは到達不可。Flowise などのコンテナからは `host.docker.internal:18080` で到達可能
- `--bind 0.0.0.0` に変更した場合は外部公開になるため README で明記する
