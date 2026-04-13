---
change_type: implementation
priority: high
dependencies: []
references:
  - src/serve.rs
  - openspec/specs/agent-exec-serve/spec.md
---

# 変更提案: POST /exec に wait / until / max_bytes を受理させる

## Problem / Context

`ExecRequest` (`src/serve.rs:125-132`) は `command` / `cwd` / `env` / `timeout_ms` のみ。inline wait は 10 秒・max-bytes 65536 に**固定** (`src/serve.rs:286`)。`wait`/`until`/`max_bytes` は MUST NOT で spec 上も拒否。

結果として、HTTP 経由 agent は短命コマンドで 10 秒待ちすぎたり、巨大 stdout を刈り込めなかったりし、CLI `run` より不便。コアコンセプト（往復削減）に逆行する。

## Proposed Solution

- `ExecRequest` に `wait: Option<bool>`（既定 true）・`until: Option<u64>`（秒、既定 10）・`max_bytes: Option<u64>`（既定 65536）・`timeout: Option<f64>`（秒、既存 `timeout_ms` を置換）を追加。
- 既存 `timeout_ms` は削除（CLI と秒単位契約を揃える）。本提案で HTTP API 契約を break change として扱う。
- spec: 「`wait` を受け付けてはならない（MUST NOT）」を撤回し、上記を MUST 受理に改定。

## Acceptance Criteria

- `POST /exec {"command":["sh","-c","exit 7"],"until":1}` が 1 秒後に終端 state を返す。
- `POST /exec {"command":[...],"wait":false}` が launch-only 返却になる。
- `POST /exec {"command":[...],"max_bytes":1024}` で inline output が 1024 バイトに制限される。
- `timeout_ms` を送ると HTTP 400（スキーマ不一致）。

## Out of Scope

- `POST /exec` の auth/CORS は `harden-serve-security` 提案で扱う。
