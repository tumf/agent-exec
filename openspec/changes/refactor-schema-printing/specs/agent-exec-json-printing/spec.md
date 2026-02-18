# agent-exec JSON 出力互換

## ADDED Requirements

### Requirement: JSON-only stdout の維持

`Response::print` と `ErrorResponse::print` は stdout に JSON オブジェクト 1 行のみを出力しなければならない（MUST）。リファクタにより余分な文字列や複数行出力が発生してはならない（MUST）。

#### Scenario: 成功レスポンスの出力
Given `Response` を生成する
When `print` を呼び出す
Then stdout は JSON 1 行のみであり、`schema_version`/`ok`/`type` を含む

#### Scenario: エラーレスポンスの出力
Given `ErrorResponse` を生成する
When `print` を呼び出す
Then stdout は JSON 1 行のみであり、`error.code`/`error.message`/`error.retryable` を含む
