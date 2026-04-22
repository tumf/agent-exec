## Implementation Tasks

- [ ] 1. `src/main.rs` に `ps` サブコマンドを追加し、`list` 実装へ `state=running` を固定して委譲する。`ps` では `--limit` / `--cwd` / `--all` / `--tag` を受け付け、`--state` は露出しないようにする (verification: integration - `tests/integration.rs` に `ps` が running のみ返すことと `--all` / `--cwd` の委譲確認を追加)
- [ ] 2. `src/main.rs` の `Delete` に `rm` alias を追加し、`delete` の既存契約をそのまま通す (verification: integration - `tests/integration.rs` に `rm <JOB_ID>` と `rm --dry-run --all` が `delete` と同等に動く回帰テストを追加)
- [ ] 3. `openspec/specs/agent-exec/spec.md` と必要なら `README.md` を更新し、`ps` と `rm` が既存 `list` / `delete` への短い到達経路であることを明記する (verification: manual - spec/README の記述が CLI surface と一致)
- [ ] 4. `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all` を実行し、CLI 契約変更が CI 相当ゲートを通ることを確認する (verification: manual - 3 コマンド成功)

## Future Work

- `ls` / `logs` / `stop` などの追加 alias は、ユーザーが必要と判断した場合に別 proposal として扱う
