# Design: deprecate-wait-timeout-ms

## Summary

`wait` サブコマンドの待機期限語彙を `--until` に統一し、実行制御の `--timeout` と観測制御の `--until` / `--forever` をユーザーが混同しない契約へ整理する。内部ロジックは既に分離されているため、この proposal の主眼は CLI 表面・ドキュメント・テストの正規経路整合にある。

## Current State

- `run/create/_supervise` の `--timeout` はジョブ実行時間の上限であり、期限到達時に TERM/KILL を送る。
- `wait` と `run --wait` の `--until` は CLI の待機上限であり、期限到達時もジョブ自体は継続する。
- canonical spec は `wait --timeout-ms` を `--until` に置換済みだが、CLI alias・README・一部テストは旧語彙を残している。

## Design Goals

- 実行タイムアウトと待機期限を異なる語彙で明確に分ける。
- canonical spec、CLI help、README、統合テストの正規経路を一致させる。
- 既存利用者への急な破壊を避ける必要がある場合のみ、legacy alias を限定維持する。

## Non-Goals

- wait のポーリングロジックやデフォルト期限を変えること。
- HTTP API の wait semantics を変えること。

## Compatibility Strategy

- 正式名称は `--until` とする。
- `--timeout-ms` を残す場合は legacy alias と明示し、新しい文書・例・正規テストからは外す。
- 後方互換を切る場合は別 proposal で完全削除を扱う。

## Verification Impact

- README と help の語彙が `timeout` と `until` で混線しないことを目視確認する。
- 統合テストでは `wait --until` を主経路として扱い、必要なら legacy alias 用の互換テストを 1 ケースだけ残す。
