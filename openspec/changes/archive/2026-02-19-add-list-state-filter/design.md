# 技術設計: list の状態フィルタ

## 方針
- 既存の `list` 実装に最小限のフィルタ処理を追加する
- 既存 JSON 形状は維持し、`jobs` の集合だけを絞り込む

## 仕様の要点
- CLI: `agent-exec list --state <state>` を追加
- `state` の値は `running|exited|killed|failed|unknown` に限定
- フィルタ → ソート → `--limit` の順で適用

## 実装メモ
- `src/main.rs` の `Command::List` に `state` フラグを追加し、`ListOpts` へ伝搬
- `src/list.rs` で `jobs` 生成後に状態フィルタを適用
- 値のバリデーションは clap の `value_parser` を利用し、未知値は usage エラーにする

## テスト方針
- `tests/integration.rs` に `list --state running` の検証を追加
- 長時間ジョブ（`sleep 60`）と短時間ジョブ（`echo`）を使って結果を比較
- テスト後に `kill` で長時間ジョブを終了する
