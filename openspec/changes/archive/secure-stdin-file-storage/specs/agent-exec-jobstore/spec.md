## ADDED Requirements

### Requirement: stdin.bin の保存仕様

`--stdin <VALUE>` / `--stdin -` / `--stdin-file <PATH>` によって materialize される入力は、job directory 直下のファイル `stdin.bin`（相対パス固定）として保存しなければならない（MUST）。`meta.json.stdin_file` はこの相対ファイル名 `"stdin.bin"` を保持しなければならない（MUST）。

Unix 系プラットフォームでは `stdin.bin` のパーミッションは `0o600` で作成しなければならない（MUST）。umask の影響を受けてはならない（MUST NOT）。Windows では NTFS ACL の既定を維持する（owner のみアクセス可能）。

書き込み時の入力サイズは既定 64 MiB（67108864 bytes）を上限としなければならない（MUST）。`--stdin-max-bytes <N>` で上限を明示指定できなければならない（MUST）。上限超過時は起動前に `error.code="stdin_too_large"` で失敗しなければならない（MUST）。

#### Scenario: stdin.bin is created with 0o600 on Unix

**Given**: a Unix-like platform
**When**: `agent-exec create --stdin "secret" -- cat` is executed
**Then**: `stdin.bin` exists inside the job directory
**And**: the file mode is `0o600`

#### Scenario: oversized stdin fails with stdin_too_large

**Given**: a 65 MiB input via `--stdin-file ./big.bin`
**When**: `agent-exec run --stdin-file ./big.bin -- cat` is executed with default `--stdin-max-bytes`
**Then**: the command fails with `error.code="stdin_too_large"` before launching the workload
