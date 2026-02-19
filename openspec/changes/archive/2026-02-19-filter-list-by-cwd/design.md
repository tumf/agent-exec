## Context

`list` は現在ルート配下の全ジョブを返すため、複数プロジェクトの混在環境では目的のジョブが埋もれやすい。`run` は `--cwd` を受け取れるが、`meta.json` には実効 cwd が保存されないため、後から cwd ベースの絞り込みができない。

## Goals / Non-Goals

**Goals:**

- ジョブ作成時の実効 cwd を `meta.json` に保存する
- `list` の既定表示を current_dir 一致に変更する
- `list --cwd <PATH>` と `list --all` を追加する

**Non-Goals:**

- 既存ジョブの `meta.json` を遡及的に修正する
- `list` の JSON ペイロードに `cwd` を追加する
- 既存の `--state`/`--limit` の挙動変更（優先順位のみ定義）

## Decisions

- **cwd の保存値:** `run` 時に実効 cwd を正規化し、`meta.json.cwd` に保存する。`--cwd` 指定時はそのパスを基準にし、未指定時は `run` 実行プロセスの current_dir を使用する。
- **パス正規化:** 保存・比較の両方で `canonicalize` を優先し、失敗時は相対パスを絶対化して文字列化する。これにより Windows の表記揺れを抑制する。
- **フィルタ優先順位:** `list --all` は cwd フィルタを無効化し、`list --cwd <PATH>` は指定パス一致のみを返す。どちらもない場合は current_dir 一致で絞り込む。
- **排他制約:** `--all` と `--cwd` は意図が衝突するため排他とし、同時指定は usage エラーとする。

## Risks / Trade-offs

- [Risk] 既存ジョブに `cwd` が無く、既定の `list` で表示されなくなる → Mitigation: `--all` を追加し全件表示の逃げ道を提供する。
- [Risk] `canonicalize` が失敗し一致判定が不安定になる → Mitigation: 失敗時は絶対化のみで比較し、保存値と比較値で同一の正規化関数を使う。

## Migration Plan

- 新規ジョブから `meta.json.cwd` を書き込む。既存ジョブは `--all` で引き続き参照可能とする。

## Open Questions

- なし
