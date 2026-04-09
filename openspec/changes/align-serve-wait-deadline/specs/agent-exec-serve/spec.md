## MODIFIED Requirements

### Requirement: GET /wait/:id による完了待機

`GET /wait/:id` は CLI `wait` サブコマンドと同等の待機意味論を提供しなければならない（MUST）。未指定時は最大 30,000ms 待機し、期限到達時にジョブが非終端状態であれば、その非終端 `state` を返して応答しなければならない（MUST）。この待機上限はジョブを停止させる timeout ではなく、HTTP リクエストが待機する最大時間でなければならない（MUST）。

`until_ms` query parameter は待機上限を上書きしなければならない（MUST）。`forever=true` は終端状態になるまで無制限に待機しなければならない（MUST）。`until_ms` と `forever=true` は同時指定を許可してはならない（MUST NOT）。

#### Scenario: default HTTP wait uses the same 30 second deadline as CLI wait

**Given**: 実行中のジョブが存在する
**When**: `GET /wait/<job_id>` をリクエストする
**Then**: 最大約 30,000ms 待機して応答する
**And**: 期限内に未完了なら `state` は `created` または `running` である
**And**: ジョブ自体は継続実行する

#### Scenario: HTTP wait supports explicit bounded deadline

**Given**: 実行中のジョブが存在する
**When**: `GET /wait/<job_id>?until_ms=100` をリクエストする
**Then**: 最大約 100ms 待機して応答する
**And**: 未完了なら `exit_code` は absent である

#### Scenario: HTTP wait supports unbounded waiting

**Given**: 実行中のジョブが存在する
**When**: `GET /wait/<job_id>?forever=true` をリクエストする
**Then**: ジョブ終了後に終端状態を含む JSON を返す

#### Scenario: HTTP wait rejects conflicting deadline controls

**Given**: 実行中のジョブが存在する
**When**: `GET /wait/<job_id>?until_ms=100&forever=true` をリクエストする
**Then**: HTTP 400 と stable JSON error を返す
