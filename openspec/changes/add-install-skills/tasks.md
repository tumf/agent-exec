## 1. スキル資材の追加

- [ ] 1.1 `skills/agent-exec/SKILL.md` を追加し埋め込み対象の最小コンテンツを用意する（確認: `skills/agent-exec/SKILL.md` が存在し英語で記述されている）
- [ ] 1.2 必要に応じて `skills/agent-exec/README.md` と `skills/agent-exec/references/` を追加する（確認: 追加したファイルが埋め込み対象に列挙されている）

## 2. スキルインストール実装

- [ ] 2.1 `src/skills.rs`（または `src/skills/mod.rs`）に Source 解析、lock 読み書き、埋め込み/ローカル展開の実装を追加する（確認: `Source::parse` と lock の読み取り互換が実装されている）
- [ ] 2.2 `src/install_skills.rs` に CLI から呼べる実行関数を追加する（確認: `Response::new("install_skills", ...)` が生成される）
- [ ] 2.3 `src/schema.rs` に `install_skills` のレスポンス payload 型を追加する（確認: `type="install_skills"` の JSON が生成できる）

## 3. CLI 統合とエラー境界

- [ ] 3.1 `src/main.rs` に `install-skills` サブコマンドを追加し `run()` で配線する（確認: `agent-exec install-skills` がルーティングされる）
- [ ] 3.2 スキルインストールの失敗を `ErrorResponse` へマッピングする（確認: 未知スキームで `error.code="unknown_source_scheme"` が返る）
- [ ] 3.3 `src/lib.rs` のモジュール公開を更新する（確認: `pub mod skills;`/`pub mod install_skills;` が追加される）

## 4. 統合テスト

- [ ] 4.1 `tests/integration.rs` に `install-skills` の成功ケースを追加する（確認: JSON が `type="install_skills"` で `skills[0].name` が `agent-exec`）
- [ ] 4.2 ローカルソース（`local:<path>`）のテストを追加する（確認: 一時ディレクトリに作成した偽スキルが `.agents/skills/<name>` に展開される）
- [ ] 4.3 未知スキームの失敗テストを追加する（確認: `error.code="unknown_source_scheme"` で exit code 1）
