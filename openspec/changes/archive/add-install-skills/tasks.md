## 1. スキル資材の追加

- [x] 1.1 `skills/agent-exec/SKILL.md` を追加し埋め込み対象の最小コンテンツを用意する（確認: `skills/agent-exec/SKILL.md` が存在し英語で記述されている）
- [x] 1.2 必要に応じて `skills/agent-exec/README.md` と `skills/agent-exec/references/` を追加する（確認: 追加したファイルが埋め込み対象に列挙されている）

## 2. スキルインストール実装

- [x] 2.1 `src/skills.rs`（または `src/skills/mod.rs`）に Source 解析、lock 読み書き、埋め込み/ローカル展開の実装を追加する（確認: `Source::parse` と lock の読み取り互換が実装されている）
- [x] 2.2 `src/install_skills.rs` に CLI から呼べる実行関数を追加する（確認: `Response::new("install_skills", ...)` が生成される）
- [x] 2.3 `src/schema.rs` に `install_skills` のレスポンス payload 型を追加する（確認: `type="install_skills"` の JSON が生成できる）

## 3. CLI 統合とエラー境界

- [x] 3.1 `src/main.rs` に `install-skills` サブコマンドを追加し `run()` で配線する（確認: `agent-exec install-skills` がルーティングされる）
- [x] 3.2 スキルインストールの失敗を `ErrorResponse` へマッピングする（確認: 未知スキームで `error.code="unknown_source_scheme"` が返る）
- [x] 3.3 `src/lib.rs` のモジュール公開を更新する（確認: `pub mod skills;`/`pub mod install_skills;` が追加される）

## 4. 統合テスト

- [x] 4.1 `tests/integration.rs` に `install-skills` の成功ケースを追加する（確認: JSON が `type="install_skills"` で `skills[0].name` が `agent-exec`）
- [x] 4.2 ローカルソース（`local:<path>`）のテストを追加する（確認: 一時ディレクトリに作成した偽スキルが `.agents/skills/<name>` に展開される）
- [x] 4.3 未知スキームの失敗テストを追加する（確認: `error.code="unknown_source_scheme"` で exit code 1）

## Acceptance #1 Failure Follow-up

- [x] `src/schema.rs` の `InstalledSkillSummary` と `src/install_skills.rs` のレスポンス生成を `source` ではなく `source_type` に変更し、`tests/integration.rs` の `install_skills_*` テストも `source_type` を検証するよう更新する（spec: `openspec/changes/add-install-skills/specs/agent-exec-skills/spec.md` Requirement: 成功レスポンスの構造）。
- [x] `src/skills.rs` の `LockEntry` と `src/install_skills.rs` の lock 更新処理を `source_type` フィールドで記録するよう修正し、`.agents/.skill-lock.json` に `name`/`path`/`source_type` が残ることを統合テストで検証する（spec: `openspec/changes/add-install-skills/specs/agent-exec-skills/spec.md` Requirement: lock ファイルの更新）。
