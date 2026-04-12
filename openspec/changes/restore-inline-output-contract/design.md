# Design: restore-inline-output-contract

## Overview

この変更は、`remove-snapshot-run-start-observation` で導入された launch-only 契約を巻き戻し、`run` / `start` の初回レスポンスに AI エージェント向けの inline output を復活させる。単なる旧 `snapshot` の復帰ではなく、field naming と観測表現を整理し、`run` / `start` の head と `tail` の tail を同じ range 契約で統一する。

## Goals

- `run` / `start` の既定呼び出しだけで短命コマンドの結果を読めるようにする
- `--no-wait` で launch-only を明示選択できるようにする
- `snapshot` / `final_snapshot` / `truncated` のような曖昧な naming を廃止する
- `run` / `start` / `tail` / serve の出力 shape を一貫させる

## Non-Goals

- ログファイルの保存形式変更
- ストリーミング API や follow モードの追加
- `status` / `wait` の大幅な role 変更

## Output Contract

### Shared fields

出力本文を含むレスポンスは次を共通で使う。

- `stdout`: UTF-8 lossy で復元した文字列
- `stderr`: UTF-8 lossy で復元した文字列
- `stdout_range`: `[begin, end]`
- `stderr_range`: `[begin, end]`
- `stdout_total_bytes`
- `stderr_total_bytes`
- `encoding`: 常に `utf-8-lossy`

### Range semantics

- range は raw log file bytes に対する 0-based offset
- JSON 形は `[begin, end]`
- 意味は half-open interval `[begin, end)`
- `returned_bytes = end - begin`
- `begin > 0` なら先頭欠落
- `end < total_bytes` なら末尾欠落

### Head vs Tail

- `run` / `start` は head を返すため、通常 `*_range[0] == 0`
- `tail` は tail を返すため、通常 `*_range[1] == *_total_bytes`
- field 名は同一だが、どの範囲を返すかはサブコマンドの責務で決まる

## Wait semantics

### Defaults

- `run` / `start` の既定は `--wait --until 10`
- `--no-wait` は `--wait --until 0` のエイリアス

### Terminal and non-terminal responses

終端した場合も非終端の場合も、同じ top-level fields を返す。
差分は `state`, `exit_code`, `finished_at` の有無だけにする。
これにより `final_output` のような別 object は不要になる。

## CLI / HTTP alignment

### CLI

- `run` / `start` は head
- `tail` は tail

### HTTP

- `POST /exec` は CLI `run` と同じデフォルト待機・field 名を使う
- `GET /tail/:id` は CLI `tail` と同じ field 名を使う

## Compatibility Notes

この変更は field 名とデフォルト待機の両方で破壊的変更になる。既存の launch-only 契約と `stdout_tail` / `stderr_tail` 契約を利用するクライアントは更新が必要。
ただし、ユーザーの要求はこれをデグレ修正として扱っており、現仕様より過去の AI エージェント向け UX を優先する。

## Verification Strategy

- `run` / `start` 既定値が 10 秒待機であることを統合テストで確認
- `--no-wait` が `--wait --until 0` と等価であることを統合テストで確認
- head は冒頭 bytes、tail は末尾 bytes を返すことを range 値込みで検証
- serve の `/exec` と `/tail/:id` が CLI と同形であることを HTTP 統合テストで検証
