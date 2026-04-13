## Implementation Tasks

- [x] Task 1: `WaitData` に `stdout_total_bytes` / `stderr_total_bytes` / `updated_at` を追加
  - verification: unit — `src/schema.rs` serde テスト
- [x] Task 2: `src/wait.rs` の満期分岐で log メトリクスと state.json から埋める
  - verification: unit — wait テスト
- [x] Task 3: 終端到達時もこれらフィールドを埋めるよう同じパスで実装する
  - verification: unit
- [x] Task 4: integration test: 長時間ジョブに対する `wait --until 1` で進捗フィールドが返ることを検証
  - verification: integration — `tests/integration.rs`
- [x] Task 5: integration test: 終端到達時にも進捗フィールドが含まれることを検証
  - verification: integration

## Future Work

- `GET /wait/:id` HTTP エンドポイントも自動で同シェイプを返す（`wait.rs` 経由のため付随的に反映されるはず）。
