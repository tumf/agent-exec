## MODIFIED Requirements

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

`run` と `create` が受け付ける definition-time option は、同じ persisted job definition を表す限り同じ metadata 意味論に落ちなければならない（MUST）。これには stdin 定義も含まれる（MUST）。`--stdin <VALUE>` と `--stdin-file <PATH>` は `run` と `create` の両方で受け付けられ、後続 `start` が追加指定なしで同じ入力を再利用できるよう persisted definition に保存されなければならない（MUST）。

`--stdin -` は呼び出し元の非対話 stdin を EOF まで読み切って materialize しなければならない（MUST）。`--stdin <STRING>` はその文字列を UTF-8 バイト列として materialize しなければならない（MUST）。`--stdin-file <PATH>` は指定ファイル内容を実行前に job directory へコピーして materialize しなければならない（MUST）。`start` は persisted stdin 定義を使って child stdin を構築し、未指定時は従来どおり null stdin を維持しなければならない（MUST）。

`--stdin -` が指定されたのに呼び出し元 stdin が tty の場合、`run` / `create` はハングせず stable API error `stdin_required` で失敗しなければならない（MUST）。`--stdin` と `--stdin-file` は同時指定を許可してはならない（MUST NOT）。

#### Scenario: run がヒアドキュメントを child stdin に渡す

Given `agent-exec run --stdin - -- cat <<'EOF'` で複数行のヒアドキュメントが渡される
When ジョブが終了する
Then 終了時の stdout ログ末尾にヒアドキュメント内容が含まれる

#### Scenario: create した stdin を start が再利用する

Given `agent-exec create --stdin "hello" -- cat` を実行する
When 後続で `agent-exec start <job_id> --wait` を実行する
Then 終了時の stdout ログ末尾に `hello` が含まれる
And `start` は追加の stdin 指定を要求しない

#### Scenario: stdin-file は materialized コピーを使う

Given `agent-exec run --stdin-file ./input.txt -- cat` を実行する
When ジョブが起動される
Then child stdin は job directory 内へコピーされた入力内容を使う
And 元の `./input.txt` パスへ実行時依存しない

#### Scenario: tty の --stdin - は即失敗する

Given 呼び出し元 stdin が tty である
When `agent-exec run --stdin - -- cat` を実行する
Then ジョブは起動前に失敗する
And `error.code` は `stdin_required` である

#### Scenario: stdin definition option は create と run で排他規則が一致する

Given `--stdin value --stdin-file ./input.txt` が指定される
When `agent-exec run` または `agent-exec create` の CLI 引数を検証する
Then どちらも usage error で失敗する
