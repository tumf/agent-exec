# 技術設計: list サブコマンド

## 目的

ジョブ保存先（root）配下のジョブを列挙し、一覧の JSON を返す。既存の stdout JSON-only 契約と終了コード規約に従う。

## 設計方針

- 既存の `resolve_root()` を使用し、保存先の優先順位を維持する
- 読み取り専用で動作し、root ディレクトリを新規作成しない
- `meta.json` が存在し、かつ JSON として読めるディレクトリのみをジョブとして扱う
- `state.json` は存在すれば読み取るが、失敗してもジョブは返す

## 出力フィールド

- `root`: 解決後の root パス
- `jobs`: ジョブ概要の配列
  - `job_id`
  - `state`: `running|exited|killed|failed|unknown`
  - `exit_code`（存在する場合のみ）
  - `started_at`（`meta.json.created_at` を使用）
  - `finished_at`（存在する場合のみ）
  - `updated_at`（`state.json.updated_at` を読めた場合）
- `truncated`: `--limit` で切り詰められた場合に true
- `skipped`: job として読めなかったディレクトリ数

## 並び順と制約

- `started_at` 降順で並べる
- 同一時刻は `job_id` 降順で安定化する
- `--limit` で上限を設定し、超過時は `truncated=true`

## 例外・エラー

- root が存在しない場合は `jobs=[]` を返して正常終了
- `read_dir` などの I/O が失敗した場合は `error` JSON を返し、終了コード 1
