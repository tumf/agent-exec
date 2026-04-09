# Design: align-serve-wait-deadline

## Summary

HTTP `GET /wait/{id}` を CLI `wait` と同じ bounded-wait contract に揃える。未指定時は 30,000ms の待機上限を使い、`until_ms` と `forever` query parameter で明示的に上書きできるようにする。これにより transport ごとの差異をなくし、`CLI equivalent` という README / spec 上の説明を実態と一致させる。

## Current State

- CLI `wait` は `src/wait.rs` で既定 30,000ms の deadline を持つ。
- `run --wait` も `--until` / `--forever` で同じ観測モデルを採用している。
- 一方 `src/serve.rs` の `wait_handler` は固定 200ms poll で無期限に待ち続ける。
- `openspec/specs/agent-exec-serve/spec.md` も `/wait/{id}` を終端まで待つものとして記述しており、CLI equivalent という表現と矛盾している。

## Design Goals

- CLI と HTTP で `wait` の意味論を一致させる。
- 既定の bounded wait を transport 非依存にする。
- query parameter だけで単純に制御できる API を維持する。
- deadline 到達時もジョブ自体は継続し、観測結果だけを返す。

## Query Model

- `GET /wait/{id}`: default deadline 30,000ms
- `GET /wait/{id}?until_ms=<N>`: explicit bounded wait
- `GET /wait/{id}?forever=true`: unbounded wait
- `until_ms` と `forever=true` の同時指定は invalid request

## Error Handling

不正な query 組み合わせは HTTP 400 を返し、既存 API 契約に合わせて stable JSON error shape を保つ。未知 job_id は引き続き HTTP 404 の `job_not_found` を返す。

## Verification Impact

- HTTP wait の default / bounded / forever の3系統を integration で確認する。
- deadline 到達時に非終端 state を返しつつジョブが継続していることを検証する。
- ドキュメント上の `/wait/{id}` 説明を、CLI `wait` と同一の semantics に揃える。
