# 設計メモ: run/tail の bytes メトリクスとログパス

## 目的

- `run`/`tail` の取得結果に、ログの観測量（bytes）と出力に含めた量（bytes）を付与する。
- `run` の待機を最大 10 秒に制限し、実測の待機時間と全体所要時間を返す。
- `stdout.log`/`stderr.log` のフルパスを JSON に含める。

## 用語

- observed_bytes: 取得時点のログファイルサイズ（bytes）。`std::fs::metadata().len()` で取得する。
- included_bytes: JSON に含める `*_tail` 文字列の UTF-8 bytes 長。`string.as_bytes().len()` を使う。
- waited_ms: `run` が `snapshot` 取得のためにブロックした実測時間。
- elapsed_ms: `run` 呼び出しの開始から JSON を出力するまでの wall-clock 所要時間。

## 仕様上の判断

- `snapshot-after` は最大 10,000ms にクランプする。`snapshot-after=0` は待機しない。
- `stdout_log_path`/`stderr_log_path` は絶対パスで返す。`canonicalize` ではなく、
  ルートの絶対パスとジョブディレクトリの結合を用いて安定したパスを返す。
- `observed_bytes` はログが存在しない場合は 0 とする。
- `included_bytes` は `utf-8-lossy` で文字列化した `*_tail` の bytes 長とする。

## 互換性

- 追加フィールドのみで既存の JSON 形は維持する。
- 既存の `truncated` 判定や `encoding` は変更しない。
