## Specification Tasks

- [x] Promote `specs/agent-exec-run-logging/spec.md` delta — full.log の扱いを明文化 (Expected canonical result: canonical spec に「full.log は人間向け、機械処理は stdout.log/stderr.log」「`\n` のみで行分割、CR および非 UTF-8 は U+FFFD lossy 置換」が記載される)
- [x] Review delta scenarios (CR 含む出力、非 UTF-8 バイト混在)

## Future Work

- `skills/agent-exec/**` の更新（機械処理ガイダンス）は別提案で実施。
