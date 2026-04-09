---
change_type: implementation
priority: high
dependencies: []
references:
  - src/serve.rs
  - src/main.rs
  - src/wait.rs
  - tests/integration.rs
  - README.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-serve/spec.md
---

# 変更提案: align-serve-wait-deadline

**Change Type**: implementation

## Premise / Context
- 現セッションでは `wait` の 30 秒既定と `--until` の意味分離を確認した上で、HTTP `GET /wait/{id}` が CLI `wait` と同じ semantics なのかが論点になった。
- canonical CLI spec では `wait` は既定 30,000ms の待機上限を持ち、`--until` / `--forever` で上書きされる。
- しかし `agent-exec-serve` の spec は `/wait/{id}` を「終端状態まで待機」とだけ記述しており、実装も無期限ブロックしている。
- そのため transport ごとに wait contract が分岐しており、CLI equivalent としての理解が成り立っていない。

## Requested Artifact
- implementation

## 背景 / 課題
`GET /wait/{id}` は README でも CLI equivalent と説明されているが、実際には CLI `wait` と異なり無期限で待機する。CLI では既定 30 秒で非終端 state を返すのに対し、HTTP ではタイムアウト概念がなく、クライアントは接続し続けるしかない。この差分は Flowise や Docker から `/wait/{id}` を使う利用者にとって重要であり、同じ “wait” という名前で transport ごとに意味が違うのは契約上危険である。

## 提案する変更
- `GET /wait/{id}` を CLI `wait` と同じ bounded-wait contract に揃える。
- HTTP wait は既定で最大 30,000ms 待機し、期限到達時にジョブが非終端なら非終端 `state` と `exit_code` absent を返す。
- HTTP wait に `until_ms` と `forever` の query parameter を追加し、CLI の `--until` / `--forever` に対応させる。
- `until_ms` と `forever` は同時指定不可とし、不正な組み合わせは HTTP 400 の stable JSON error で返す。
- `/wait/{id}` の README と serve spec を、CLI equivalent が transport をまたいで同一の wait semantics を持つよう更新する。

## Acceptance Criteria
- `GET /wait/{id}` は未指定時に最大 30,000ms 待機し、未完了なら非終端 `state` を返す。
- `GET /wait/{id}?until_ms=100` は最大 100ms 待機し、ジョブが未完了なら `state=created|running` と `exit_code` absent を返す。
- `GET /wait/{id}?forever=true` はジョブが終端状態になるまで待機する。
- `GET /wait/{id}?until_ms=100&forever=true` は HTTP 400 と stable JSON error を返す。
- README と serve spec の `/wait/{id}` 説明が、CLI `wait` と同一の semantics を明示する。

## Out of Scope
- `/status/{id}` や `/tail/{id}` の意味論変更
- POST `/exec` の request body に wait deadline を追加すること
- server-side cancellation や streaming response の導入
