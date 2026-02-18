# agent-exec-run Specification (Delta)

## MODIFIED Requirements

### Requirement: run の監視分離

MUST: `run` は `snapshot-after` で指定された待機期限までブロックし、
ジョブが `running` のままであれば期限到達まで待機を継続しなければならない（MUST）。
出力が既に存在する場合でも、ジョブが継続中であれば待機を短縮してはならない（MUST）。
ただしジョブが終了した場合は期限より前に返却してよい（MAY）。

#### Scenario: 出力が先に出ても期限まで待つ

Given `agent-exec run --snapshot-after 200 -- sh -c "printf 'hi'; sleep 1"` を実行する
When `run` の JSON が返る
Then `waited_ms` は 200 以上である
