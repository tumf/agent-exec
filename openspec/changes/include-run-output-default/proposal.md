# 変更提案: run の出力に stdout/stderr を既定で含める

## 目的

`agent-exec run` の JSON 出力に、子プロセスの `stdout`/`stderr` を `max_bytes` 上限内で既定で含め、短命なコマンドでも実際の出力が返る状態にする。

## 背景

現状の `run` は `snapshot_after > 0` のときのみ `snapshot` を返し、さらに出力のタイミング次第で `stdout_tail`/`stderr_tail` が空になることがある。これにより、短命コマンドの出力が `run` の JSON に含まれず、期待と乖離する。

## スコープ

- `run` のデフォルトで `snapshot` が返るようにする
- `snapshot` が `max_bytes` 以内の `stdout`/`stderr` を含める
- 改行なしの出力も `snapshot` に反映されるようにする
- 既存の JSON フィールド名やログファイル構成は維持する

## スコープ外

- `tail` コマンドの仕様変更
- 既存のログ出力先（stdout.json/stderr.log/full.log）のパス構成変更
- `run` が返す JSON 以外の出力形式追加

## リスクと互換性

- `run` のデフォルト応答に `snapshot` が含まれるため、JSON サイズが増える
- `run` の待機が既定で短時間入るため、`elapsed_ms` がわずかに増える
- 既存フィールドの名称・構造は維持し、互換性を確保する

## 成功条件

- `agent-exec run -- <cmd>` のデフォルト実行で `snapshot` が返る
- 短命コマンドの `stdout` が `snapshot.stdout_tail` に含まれる
- 改行なしの出力でも `snapshot.stdout_tail` に反映される
