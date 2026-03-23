## MODIFIED Requirements

### Requirement: JSON-only stdout の維持

`Response::print` と `ErrorResponse::print` は、選択された出力フォーマットに応じて stdout に機械可読なレスポンスを 1 つだけ出力しなければならない（MUST）。既定では JSON オブジェクト 1 行のみを出力しなければならない（MUST）。`--yaml` が指定された場合は、同一レスポンス内容を YAML ドキュメントとして出力しなければならない（MUST）。どちらの形式でも `schema_version`/`ok`/`type` を含むレスポンス構造を保たなければならない（MUST）。

#### Scenario: 既定成功レスポンスの出力
Given `Response` を生成する
When 既定フォーマットで `print` を呼び出す
Then stdout は JSON 1 行のみであり、`schema_version`/`ok`/`type` を含む

#### Scenario: YAML 成功レスポンスの出力
Given `Response` を生成する
When YAML フォーマットで `print` を呼び出す
Then stdout は YAML ドキュメントであり、構造化データとして `schema_version`/`ok`/`type` を含む

#### Scenario: YAML エラーレスポンスの出力
Given `ErrorResponse` を生成する
When YAML フォーマットで `print` を呼び出す
Then stdout は YAML ドキュメントであり、`error.code`/`error.message`/`error.retryable` を含む
