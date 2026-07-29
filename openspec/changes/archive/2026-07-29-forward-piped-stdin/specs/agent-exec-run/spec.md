## MODIFIED Requirements

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

`run` と `create` が受け付ける明示的な definition-time option は、同じ persisted job definition を表す限り同じ metadata 意味論に落ちなければならない（MUST）。これには明示的な stdin 定義も含まれる（MUST）。`--stdin <VALUE>` と `--stdin-file <PATH>` は `run` と `create` の両方で受け付けられ、後続 `start` が追加指定なしで同じ入力を再利用できるよう persisted definition に保存されなければならない（MUST）。

`--stdin -` は呼び出し元の非対話 stdin を EOF まで読み切って materialize しなければならない（MUST）。`--stdin <STRING>` はその文字列を UTF-8 バイト列として materialize しなければならない（MUST）。`--stdin-file <PATH>` は指定ファイル内容を実行前に job directory へコピーして materialize しなければならない（MUST）。`start` は persisted stdin 定義を使って child stdin を構築し、未指定時は従来どおり null stdin を維持しなければならない（MUST）。

`run` は `--stdin` と `--stdin-file` のどちらも指定されず、呼び出し元 stdin が非 tty の場合、その stdin を EOF まで自動的に読み取り、`--stdin -` と同じ上限付き `stdin.bin` materialization と `meta.json.stdin_file` 契約を適用しなければならない（MUST）。明示的な `--stdin` または `--stdin-file` がある場合は明示指定を優先し、暗黙 stdin を読み取ってはならない（MUST NOT）。呼び出し元 stdin が tty の場合、stdin option 未指定の `run` は terminal を読み取らず child stdin を null のまま維持しなければならない（MUST）。

`create` は stdin option 未指定時に呼び出し元 stdin を暗黙 capture してはならない（MUST NOT）。`create` の stdin definition は `--stdin -`、`--stdin <STRING>`、または `--stdin-file <PATH>` による明示指定を必要とする（MUST）。

`--stdin -` が指定されたのに呼び出し元 stdin が tty の場合、`run` / `create` はハングせず stable API error `stdin_required` で失敗しなければならない（MUST）。`--stdin` と `--stdin-file` は同時指定を許可してはならない（MUST NOT）。明示または暗黙に materialize する入力が `--stdin-max-bytes` を超える場合、ジョブ起動前に stable API error `stdin_too_large` で失敗しなければならない（MUST）。

#### Scenario: run が暗黙にパイプ入力を child stdin に渡す

**Given**: `printf 'alpha\nbeta\n' | agent-exec run -- cat` が stdin option なしで実行される
**When**: ジョブが終了する
**Then**: child stdout は入力 bytes を保持した `alpha\nbeta\n` である
**And**: job directory の `stdin.bin` は同じ bytes を含む
**And**: `meta.json.stdin_file` は `stdin.bin` を示す

#### Scenario: 明示 stdin は暗黙パイプ入力より優先される

**Given**: 非 tty stdin が接続された `agent-exec run --stdin explicit -- cat` が実行される
**When**: stdin source が解決される
**Then**: child stdin は `explicit` である
**And**: 呼び出し元の暗黙 stdin は materialize されない

#### Scenario: stdin option なしの tty run は terminal を読まない

**Given**: 呼び出し元 stdin が tty である
**When**: `agent-exec run -- cat` が実行される
**Then**: `run` は terminal input を待たない
**And**: child stdin は null である

#### Scenario: 暗黙 stdin にもサイズ上限を適用する

**Given**: `--stdin-max-bytes` を超える非 tty stdin が stdin option なしの `run` に渡される
**When**: 入力を materialize する
**Then**: ジョブは起動前に失敗する
**And**: `error.code` は `stdin_too_large` である

#### Scenario: create は stdin を暗黙 capture しない

**Given**: 非 tty stdin が接続され、stdin option が指定されていない
**When**: `agent-exec create -- cat` が実行される
**Then**: 作成された job definition は stdin file を持たない
**And**: 後続 `start` の child stdin は null である
