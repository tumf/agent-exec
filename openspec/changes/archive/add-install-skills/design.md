## Context

agent-exec はジョブ実行用 CLI であり、標準出力は JSON-only の契約を持ちます。slack-rs では `install-skills` が組み込まれており、埋め込みスキルまたはローカルディレクトリから `.agents/skills/` に展開し、`.skill-lock.json` で追跡しています。本変更では agent-exec に同等の挙動を導入し、既存契約（JSON エンベロープ、非対話、終了コード）を維持します。

## Goals / Non-Goals

**Goals:**
- `install-skills` の入力解釈、展開、lock 更新を CLI とライブラリで提供する
- 成功/失敗の JSON を既存エンベロープに統合する
- テストはプロジェクトローカルの `.agents` 配下で完結させる

**Non-Goals:**
- リモート（GitHub/URL）からのスキル取得
- グローバルインストールの自動テスト（ホームディレクトリ汚染を避ける）
- スキル内容の動的検証や署名検証

## Decisions

- **埋め込みスキルの提供方法**: `skills/agent-exec/**` を `include_bytes!` でバイナリに埋め込み、`self` ソースの既定とする。依存追加が不要で、実行環境に外部ファイルを要求しない。
- **ソース解釈**: `self` と `local:<path>` のみを受け付け、その他は `unknown_source_scheme` で即時失敗。曖昧な入力を許容しないことで CLI の安全性と再現性を保つ。
- **展開先解決**: `--global` なしは `<cwd>/.agents`、`--global` は `~/.agents`。slack-rs と同じ配置に揃え、ユーザーの期待と互換性を優先する。
- **ローカル展開方式**: Unix では symlink を試し、失敗時は再帰コピーにフォールバック。Windows はコピーのみ。開発効率と互換性を両立する。
- **lock 形式互換**: 配列形式を正とし、旧マップ形式の読み取り互換を持つ。既存ツールとの相互運用性を担保する。
- **JSON レスポンス**: `schema_version=0.1`, `type=install_skills` のエンベロープに `skills`/`global`/`lock_file_path` を追加。既存 contract に準拠したまま結果を表現する。

## Risks / Trade-offs

- **[ホームディレクトリへの書き込み]** → `--global` を明示オプションにし、テストはローカルインストールのみで検証する
- **[symlink 不可環境での挙動差]** → フォールバックコピーを必須化し、差分は性能のみとする
- **[lock 形式の互換ロジック追加]** → 読み取り互換に限定し、書き込みは配列形式へ統一する
