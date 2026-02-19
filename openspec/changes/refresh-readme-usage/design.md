# Design: refresh-readme-usage

## 目的
README を実装に合わせ、初見ユーザーが最短で試せる導線を提供する。

## 構成方針
- **最小の3フロー**: 短命/長命/タイムアウトの代表例に絞る
- **JSON-only 明記**: stdout は JSON のみ、stderr はログであることを明記
- **コピペ可能**: 例はそのまま実行できる形にする

## 追加する例の概要
- 短命: `run --wait` で exit_code を取得
- 長命: `run` → `status` → `tail`
- タイムアウト: `run --timeout --kill-after`
