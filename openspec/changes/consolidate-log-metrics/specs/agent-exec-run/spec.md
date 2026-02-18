# agent-exec-run Specification (Delta)

## MODIFIED Requirements

### Requirement: run と tail の bytes メトリクスの一貫性

MUST: `run` の `snapshot` と `tail` は、`stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を同一の算出規則に基づいて返さなければならない。
MUST: 算出規則は既存要件に従い、`observed_bytes` は取得時点のログファイルサイズ、
`included_bytes` は JSON に含めた `*_tail` の UTF-8 bytes 長を示す。

#### Scenario: bytes メトリクスの一貫性

Given 同一ジョブに対して `run` の `snapshot` と `tail` を取得する
When 取得時点のログファイルサイズが観測される
Then `run` と `tail` の `*_observed_bytes` と `*_included_bytes` は同一の規則で算出される
