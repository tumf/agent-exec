## Implementation Tasks

- [ ] 1. `src/main.rs` の `install-skills` サブコマンドに `--claude` を追加し、help text を `.agents` / `.claude` 切替前提へ更新する（verification: unit - `src/main.rs` の clap 定義が `claude: bool` を受け取り `install_skills::InstallSkillsOpts` へ渡す）
- [ ] 2. `src/skills.rs` のインストール先解決を一般化し、`.agents` と `.claude` の両ルートで `skills/` と `.skill-lock.json` を返せるようにする（verification: unit - ルート解決ヘルパーが local/global × agents/claude の4通りを表現できる）
- [ ] 3. `src/install_skills.rs` と関連 schema コメントを更新し、選択されたルート配下にインストールと lock 更新を行う（verification: integration - `tests/integration.rs` で `skills[0].path` と `lock_file_path` が選択ルートを指す）
- [ ] 4. `tests/integration.rs` に `--claude` のローカル/グローバル成功ケースを追加し、既存 `.agents` ケースの後方互換も維持する（verification: integration - `cargo test --test integration install_skills`）
- [ ] 5. `openspec/specs/agent-exec-skills/spec.md` と proposal delta を反映する実装整合性を確認し、fmt/clippy/test を通す（verification: integration - `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all`）

## Future Work

- 既存ユーザーの `.agents` から `.claude` への移行支援
- 他の skill 管理操作が追加された場合の `.claude` ルート共有
