# 変更提案: run の既定 snapshot-after を 10,000ms に変更

## 背景

`agent-exec run` は既定で即時返却するため、継続実行中のプロセスでも
十分な出力が含まれないケースがある。既定で 10 秒待機させることで、
スナップショットの有用性を高める。

## 目的

- `run` の既定 `snapshot-after` を 10,000ms に変更する。
- `snapshot-after=0` 指定時は即時返却（snapshot 省略可）を維持する。

## スコープ

- `agent-exec run` の既定待機時間と関連テスト更新

## 非スコープ

- `snapshot-after` の上限（10,000ms クランプ）の変更
- `tail`/`status`/`wait`/`kill` の挙動変更

## 仕様変更の要約

- 既定の待機時間を 10,000ms とする。
- `snapshot-after=0` の挙動は変更しない。

## 検証方針

- 既定 `run` で `snapshot` が返ることを統合テストで確認する。
- 既定待機が 10 秒以内であることを `waited_ms` の値で検証する。
