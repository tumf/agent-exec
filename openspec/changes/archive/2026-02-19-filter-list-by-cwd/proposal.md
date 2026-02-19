## Why

`list` は現在ルート配下の全ジョブを返すため、複数プロジェクトを横断する環境では目的の実行履歴を絞り込めず、探索コストが高い。実行時のカレントディレクトリ情報を保存し、呼び出し元のディレクトリに一致するジョブだけを既定表示することで、日常の利用効率を高める。

## What Changes

- `run` がジョブ作成時の実効カレントディレクトリを `meta.json` に保存する
- `list` の既定挙動を「呼び出し元 current_dir と一致するジョブのみ表示」に変更する
- `list --cwd <PATH>` を追加し、指定ディレクトリ実行のみを表示できるようにする
- `list --all` を追加し、cwd フィルタを無効化できるようにする

## Capabilities

### New Capabilities

### Modified Capabilities

- `agent-exec`: `list` の既定フィルタと `--cwd/--all` の挙動を追加
- `agent-exec-jobstore`: `meta.json` に `cwd` を保存

## Impact

- CLI: `list` に `--cwd`/`--all` を追加（排他オプション）
- 永続化: `meta.json` に `cwd` フィールド追加
- 実装: `src/run.rs`, `src/list.rs`, `src/main.rs`, `src/schema.rs`
- テスト: `tests/integration.rs`
