---
change_type: implementation
priority: medium
dependencies: []
references:
  - AGENTS.md
  - src/main.rs
  - src/run.rs
  - src/wait.rs
  - openspec/specs/agent-exec-run/spec.md
  - tests/integration.rs
---

# 変更提案: simplify-wait-time-units

**Change Type**: implementation

## Premise / Context
- 現セッションでは `wait --until` と `poll_ms` の単位が人間向け CLI として不自然で、ミリ秒指定は利用価値が低いという要求が明示された。
- 現行実装は `run --wait` と `wait` の両方で `--until` / `poll_ms` をミリ秒として受け付けており、既定値は `--until=30000`、`poll_ms=200` である (`src/main.rs:300`-`306`, `src/main.rs:386`-`394`, `src/wait.rs:17`-`32`)。
- リポジトリの agent guide では CLI 契約変更時に clap surface、統合テスト、関連仕様を一緒に更新し、`cargo fmt` / `cargo clippy` / `cargo test` か `prek run -a` で検証することが求められている (`AGENTS.md:141`-`147`)。
- canonical spec にはまだ `--until <ms>` が残っているため、今回の変更は仕様・実装・テスト・ヘルプの一括更新を伴う CLI 契約変更である。
- 既存の OpenSpec には `wait --timeout-ms` 廃止の履歴があり、待機系オプションは混乱しやすいため、今回も「正規語彙を一貫させる」ことが重要になる。

## Requested Artifact
- implementation

## 背景 / 課題
現在の `agent-exec` は、人が直接叩く待機系オプションに対してミリ秒単位を露出している。`wait --until 30000` や `--poll-ms 200` は内部表現としては扱いやすいが、CLI 利用者にとっては読みにくく、秒単位で考える自然な操作感とずれている。さらに、ポーリングは OS スケジューリングやファイル更新タイミングに依存するため、ミリ秒粒度を指定できてもその精度を約束できない。

このズレは単なる好みではなく、CLI 契約のわかりやすさと誤設定耐性に関わる。人間向けオプションが `30` ではなく `30000` を要求することで、指定ミスや読み違いが起きやすくなる。加えて、`run --wait` と `wait` の両方に同じ概念があるため、単位体系が不自然なままだとヘルプ、ドキュメント、統合テスト、将来の互換判断まで複雑化する。

## 提案する変更
- `run --wait` と `wait` の `--until` を秒単位へ変更し、デフォルト値も意味論上は 30 秒として扱う。
- `wait` の `--poll-ms` を `--poll` に置き換え、秒単位のポーリング間隔を受け付ける。`run --wait` 側の `wait_poll_ms` も同様に人間向けオプション名・単位へ揃える。
- 内部実装は必要に応じて `Duration` やミリ秒へ変換してよいが、CLI 契約・ヘルプ・README・統合テストでは秒単位を正規表現とする。
- 既存のミリ秒指定オプションは canonical surface から外し、必要なら usage error として拒否する。少なくとも正規ドキュメントと統合テストではミリ秒表記を支持しない。
- OpenSpec に「人が指定する待機・ポーリング時間は秒単位」という要件を追加し、`run --wait` と `wait` の両方で一貫した受理・既定値・排他制約を定義する。

## Acceptance Criteria
- `agent-exec wait --until 30 <job_id>` は最大約 30 秒待機する秒単位の正規指定として扱われる。
- `agent-exec run --wait --until 30 -- ...` も同じく秒単位の待機期限として扱われる。
- `wait` のポーリング間隔は秒単位の正規フラグで指定でき、既定値も人間向けに理解しやすい秒表現になる。
- canonical spec・clap help・README・統合テストは待機/ポーリングの正規単位を秒として一貫して案内する。
- 旧ミリ秒フラグや旧ミリ秒解釈を残す場合は、その互換方針と拒否/移行挙動が spec と統合テストで明示される。
- 実装時の検証計画に `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` または `prek run -a` が含まれる。

## Out of Scope
- ジョブ実行時間そのものを制限する `--timeout` の意味変更
- HTTP API のレスポンス JSON 形状変更
- 秒未満精度の厳密なスケジューリング保証
