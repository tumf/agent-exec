---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/run.rs
  - src/create.rs
  - src/start.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-jobstore/spec.md
---

# 変更提案: add-run-job-stdin-input

**Change Type**: implementation

## Premise / Context
- このセッションでは、`agent-exec run` でケース A のヒアドキュメントをジョブ本体へ渡したい、という要望が明示された。
- 現状の実装では `src/run.rs` の supervisor 起動と子プロセス起動の両方で stdin が `Stdio::null()` に固定されており、呼び出し元 stdin はジョブへ届かない。
- `src/run.rs:25-29` の定義時オプション整合ルールにより、新しい persisted metadata は `run` と `create` の両方で同じ意味論を持つ必要がある。
- 既存の `create` / `start` ライフサイクルでは、実行定義は `meta.json` に永続化され、`start` がそれを再利用して起動する前提がある。
- この変更はストリーミング stdin ではなく、ジョブ起動前に入力を materialize して後続起動でも再利用できる非対話的 stdin サポートとして設計する。

## Requested Artifact
- implementation

## 背景 / 課題
`agent-exec` は非対話ジョブランナーとして stdout JSON 契約を重視している一方で、ジョブに stdin を渡す手段がない。そのため、`agent-exec run --stdin - -- cat <<'EOF' ... EOF` のような自然な利用形が成立せず、ヒアドキュメント、pipe、`create`→`start` の遅延実行でも同じ入力を再利用できない。現在の `run` 実装は呼び出し元 stdin を supervisor や子プロセスへ一切接続しないため、この制約は shell quoting ではなく CLI 契約上の欠落である。

## 提案する変更
- `run` と `create` に `--stdin <VALUE>` と `--stdin-file <PATH>` を追加する。
- `--stdin -` は `agent-exec` 自身の stdin を読み取り、ヒアドキュメント / pipe / redirect の入力源として扱う。
- `--stdin <STRING>` はインライン文字列をそのままジョブ stdin 内容として扱う。
- `--stdin-file <PATH>` は指定ファイルの内容をジョブディレクトリ内へコピーし、元ファイルではなく materialized なコピーを実行時に使う。
- すべての stdin 入力源はジョブディレクトリ内の `stdin.bin` に materialize し、`meta.json` に `stdin_file` を保存して `start` と `run` の両方で同じ起動経路を使う。
- `start` は新しい stdin 定義フラグを受け付けず、`create` 時に保存された `stdin_file` を使ってジョブを起動する。
- `--stdin -` 指定時に呼び出し元 stdin が tty の場合はハング防止のため即エラーとし、`error.code="stdin_required"` を返す。
- `--stdin` と `--stdin-file` は clap usage error レベルで排他にする。
- `--stdin*` 未指定時は後方互換のため従来どおり null stdin を維持する。

## Acceptance Criteria
- `agent-exec run --stdin - -- cat <<'EOF' ... EOF` はジョブを正常終了させ、ヒアドキュメント内容を `stdout.log` / `tail` から観測できる。
- `printf 'abc' | agent-exec run --stdin - -- cat` は `abc` をジョブ stdout に出力する。
- `agent-exec run --stdin "abc" -- cat` は `abc` をそのまま stdin として渡し、暗黙の改行を追加しない。
- `agent-exec run --stdin-file /tmp/in.txt -- cat` は `/tmp/in.txt` の内容をジョブ stdin として使い、実行時にはジョブディレクトリ内へ materialize されたコピーを参照する。
- `agent-exec create --stdin "hello" -- cat` の後に `agent-exec start --wait <job_id>` を実行すると、`hello` が stdin として渡される。
- `agent-exec create --stdin - -- cat <<'EOF' ... EOF` は `create` 時に stdin を読み切って保存し、後続 `start` が追加入力なしで同じ内容を利用できる。
- `agent-exec run --stdin - -- cat` を tty から pipe / redirect なしで実行した場合、即失敗し `error.code="stdin_required"` を返す。
- `agent-exec run --stdin x --stdin-file /tmp/in.txt -- cat` と `agent-exec create --stdin x --stdin-file /tmp/in.txt -- cat` は usage error になる。
- `--stdin*` 未指定の `run` / `create` / `start` の挙動は既存どおり null stdin を維持する。

## Out of Scope
- 長時間生きる対話的 stdin や supervisor 経由のライブストリーミング入力
- stdin 内容の masking や secret redaction の導入
- `run` / `start` の JSON レスポンスへ stdin 本文や stdin パスを追加すること
- shell wrapper、notification、timeout の意味論変更
