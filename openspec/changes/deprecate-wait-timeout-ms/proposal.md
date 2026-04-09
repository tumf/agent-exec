---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/wait.rs
  - tests/integration.rs
  - README.md
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: deprecate-wait-timeout-ms

**Change Type**: implementation

## Premise / Context
- 現セッションでは `agent-exec wait` が 30 秒で返る挙動を確認する中で、プロセス実行制御の `--timeout` と待機期限の `--until` が混同されやすいことが問題として明示された。
- canonical spec では `wait` サブコマンドの旧 `--timeout-ms` を `--until` に置換することを既に要求している (`openspec/specs/agent-exec-run/spec.md`)。
- 現状の CLI 実装では `wait` に `--timeout-ms` alias が残っており、README と統合テストも旧語彙を使っているため、仕様意図と表面 API がずれている。
- `run` 系では `--timeout` が「ジョブを止める実行タイムアウト」を意味しており、`wait` 側の待機期限語彙と明確に分離した方が CLI 契約が理解しやすい。

## Requested Artifact
- implementation

## 背景 / 課題
`agent-exec` には「ジョブを何 ms で停止するか」を表す `--timeout` と、「CLI が何 ms 待ってから返るか」を表す `--until` の 2 種類の時間制御がある。内部実装と canonical spec はこの分離を前提にしているが、`wait` にだけ旧名 `--timeout-ms` が alias として残っているため、利用者からは `run --timeout` と `wait --timeout-ms` が同種の制御に見えてしまう。さらに README とテストが旧語彙を使い続けているため、設計意図が表面に現れていない。

## 提案する変更
- `wait` サブコマンドの正式な待機期限オプションを `--until` に統一し、CLI help・README・統合テストの例をすべて `--until` ベースへ更新する。
- `--timeout-ms` は互換維持が必要なら deprecation 扱いとして限定的に残すが、少なくとも新規ドキュメントと正規テスト経路からは除外する。
- `run --timeout` はジョブ実行制御、`wait --until` / `run --wait --until` は観測上の待機期限という役割分離を help 文言に明記する。
- 互換 alias を残す場合でも、その意味が「wait deadline」でありジョブ停止タイムアウトではないことを明示する。

## Acceptance Criteria
- `agent-exec wait --until 100 <job_id>` が待機期限オプションの正規例として README とテストで使われる。
- `agent-exec wait <job_id>` の既定 30,000ms 待機挙動は維持される。
- `agent-exec wait --forever <job_id>` の意味論は維持される。
- `run --timeout <ms>` と `wait --until <ms>` の役割差が CLI help または README 上で明示される。
- 互換のため `--timeout-ms` を残す場合、その扱いが deprecated / legacy alias として文書化され、正規の使用例からは除外される。

## Out of Scope
- ジョブ実行タイムアウト (`run/create/_supervise` の `--timeout`) の意味変更
- `wait` の既定 30 秒待機ロジックそのものの変更
- HTTP `GET /wait/{id}` の意味論変更（別 proposal で扱う）
