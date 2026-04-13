## Implementation Tasks

- [ ] 1. `src/main.rs` の `Run` / `Start` にある `--wait` の clap 定義を、裸指定を `true` として受理しつつ `--wait true|false` も後方互換で扱える surface へ変更する (verification: integration - `tests/integration.rs` に `run/start --wait` 裸指定と `--wait true|false` の受理テストを追加)
- [ ] 2. `run` / `start` の effective wait 計算が `--no-wait`, `--until`, `--forever` と現行どおり整合することを確認し、必要なら引数解釈を最小限調整する (verification: integration - `tests/integration.rs` で `--wait`, `--wait false`, `--no-wait`, 排他条件を確認)
- [ ] 3. canonical spec / README / skills / help 文言を更新し、`--wait` の主契約を裸指定ベースへ揃え、明示 bool 形式は後方互換として記述する (verification: manual - `openspec/specs/agent-exec/spec.md`, `openspec/specs/agent-exec-run/spec.md`, `README.md`, `skills/agent-exec/SKILL.md` の記述一致を確認)
- [ ] 4. `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all` を実行し、CLI 契約変更後の CI 相当ゲートを通す (verification: manual - 3 コマンド成功)

## Future Work

- `serve` HTTP API でも CLI 同様に「フラグ的 wait surface」を提供したい場合は別 proposal で扱う
