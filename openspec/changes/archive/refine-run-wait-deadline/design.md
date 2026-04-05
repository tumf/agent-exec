# Design: refine-run-wait-deadline

## 目的
`run --wait` と `wait` サブコマンドの待機制御を統一し、「既定は短時間待機、明示で上書き」という一貫した体験を提供する。ジョブ timeout の概念とは分離したまま表現する。

## 設計判断
- `--timeout` は既存どおりジョブ実行時間の制限として維持する。
- `run --wait` と `wait` の待機制御には新しい観測用オプション `--until` / `--forever` を追加する。
- 無制限待機は `--forever` で明示し、既定動作は 30,000ms に変更する。
- 待機期限に達してもジョブは殺さず、呼び出しだけを返す。
- `final_snapshot`・`finished_at`・`exit_code` は終端状態まで到達した場合のみ含める。

## CLI 意味論

### `run` サブコマンド
| 指定 | 待機上限 |
|------|----------|
| `run --wait` | 30,000ms |
| `run --wait --until <ms>` | `<ms>` |
| `run --wait --forever` | 無制限 |
| `run --timeout <ms>` | ジョブ timeout（待機とは無関係） |

### `wait` サブコマンド
| 指定 | 待機上限 |
|------|----------|
| `wait <job_id>` | 30,000ms |
| `wait --until <ms> <job_id>` | `<ms>` |
| `wait --forever <job_id>` | 無制限 |

### 既存 `--timeout-ms` の扱い
- `wait --timeout-ms` は `--until` へ置換する。
- 後方互換が必要であれば hidden alias として残す。この提案では alias なしの置換を推奨する。

## バリデーション制約
- `run` 側: `--until` / `--forever` は `--wait` 必須。
- `wait` 側: `--until` / `--forever` はそのまま使える（`wait` 自体が待機コマンドなため `--wait` は不要）。
- `--until` と `--forever` は常に同時指定不可。

## レスポンス方針
### 終端状態まで待てた場合
- `state` は `exited|killed|failed`
- `finished_at`、`final_snapshot`（`run --wait` のみ）、`exit_code` を含める
- `waited_ms` は実待機時間を返す

### 期限までに終わらなかった場合
- `state` は `created|running`
- `finished_at` / `final_snapshot` / `exit_code` は含めない
- `waited_ms` は期限まで待った実時間を返す

## 影響範囲
- `src/main.rs`: clap オプションと制約追加（`Run` + `Wait`）
- `src/run.rs`: `run_snapshot_wait` の deadline 導入
- `src/wait.rs`: `WaitOpts` を `--until` / `--forever` 体系に移行
- `tests/integration.rs`: 両サブコマンドの期限内終了・期限超過・forever・usage error を追加
- `openspec/specs/agent-exec-run/spec.md`: canonical requirement の更新
