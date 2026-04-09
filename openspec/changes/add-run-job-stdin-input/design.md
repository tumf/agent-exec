# Design: add-run-job-stdin-input

## Summary

`agent-exec` にジョブ stdin 入力サポートを追加する。入力源は `--stdin -`、`--stdin <STRING>`、`--stdin-file <PATH>` の 3 形態だが、内部的にはすべてジョブディレクトリ内の materialized ファイルへ統一する。これにより `run` と `create` / `start` が同じ実行モデルを共有し、呼び出し元 stdin の寿命や後続 `start` 実行時の環境差異に依存しない。

## Current State

- `src/run.rs` の `spawn_supervisor_process` は `_supervise` に対して stdin を `Stdio::null()` で固定している。
- `src/run.rs` の `supervise` は実際の子プロセスも `child_cmd.stdin(Stdio::null())` で起動する。
- `create` / `start` では `meta.json` に保存した definition-time metadata を `start` が再利用するが、stdin 入力定義は保存されていない。
- `meta.json` は run/create の共有定義を保存する前提があり、新しい definition-time metadata を追加する場合は両経路を揃える必要がある。

## Design Goals

- 非対話のままヒアドキュメント、pipe、inline 文字列、file 入力を扱えること。
- `run` と `create` / `start` で同じ persisted definition を使えること。
- デフォルト挙動を変えず、`--stdin*` 未指定時は null stdin を維持すること。
- ハングしやすい tty 読み取りを明示的に拒否すること。

## Input Model

### Supported Sources

- `--stdin -`
  - `agent-exec` 自身の stdin を EOF まで読み切る。
  - ヒアドキュメント、pipe、redirect を想定する。
- `--stdin <STRING>`
  - CLI 引数の文字列を UTF-8 バイト列として保存する。
- `--stdin-file <PATH>`
  - 指定ファイルの内容をコピーする。

### Canonical Internal Form

すべての入力源は `<job-dir>/stdin.bin` に materialize する。`meta.json` には `stdin_file` を保存し、`run` と `start` はその値をもとに子プロセス stdin を組み立てる。

この方式を選ぶ理由:

- `run` の front-end は supervisor 起動後すぐ返るため、front-end stdin を直接 supervisor/child に引き継ぐ設計は寿命が合わない。
- `create --stdin -` を許可するには、入力内容を `start` 前に永続化しておく必要がある。
- 実行時に元ファイルや呼び出し元 pipe の存在へ依存しない。
- テストとデバッグで `stdin.bin` の有無を確認しやすい。

## Metadata Changes

`meta.json` に以下を追加する:

- `stdin_file: Option<String>`

値は job dir 相対パスを推奨する。未指定時は `None` とし、既存ジョブとの後方互換を保つ。

`state.json` には新しいフィールドを追加しない。stdin は実行定義であり、進行状態ではないため。

## Command Semantics

### `run`

- `run --stdin -` は supervisor 起動前に stdin を読み切って `stdin.bin` を作る。
- `run --stdin <STRING>` はその値を `stdin.bin` に保存する。
- `run --stdin-file <PATH>` は指定ファイルを `stdin.bin` にコピーする。
- materialize 成功後にのみ supervisor を起動する。

### `create`

- `create` でも `run` と同じ definition-time フラグを受け付ける。
- `create --stdin -` は `create` 時点で stdin を読み切る。
- `create` は `stdin.bin` と `meta.json.stdin_file` を書くが、ジョブ実行はしない。

### `start`

- `start` は新しい stdin フラグを受け付けない。
- `meta.json.stdin_file` が存在する場合、そのファイルを open して子プロセス stdin に接続する。
- 未指定なら従来どおり null stdin。

## TTY Safety

`--stdin -` が指定されたのに `agent-exec` 自身の stdin が tty の場合は即エラーとする。これは対話入力待ちによるハングを避け、`--stdin -` を「非対話の pipe / heredoc / redirect 専用」と明確化するため。

推奨 API error:

- `error.code = "stdin_required"`
- `retryable = false`

## File Handling

- `stdin.bin` はバイナリファイルとして扱い、改行変換やエンコーディング変換をしない。
- `--stdin <STRING>` のみ UTF-8 文字列入力からバイト列へ変換する。
- `--stdin-file` は参照保持ではなくコピーとすることで、元ファイルの後続変更・削除・権限差異の影響を避ける。

## Verification Impact

統合テストで最低限以下を証明する必要がある:

1. `run --stdin -` がヒアドキュメントと pipe をジョブへ渡せる。
2. `run --stdin <STRING>` が改行を付加せずに入力を渡せる。
3. `run --stdin-file` が materialized コピーを使う。
4. `create --stdin*` と `start` が同じ persisted definition を再利用できる。
5. `--stdin -` の tty 実行が即エラーになる。
6. `--stdin*` 未指定時の既存挙動が維持される。
