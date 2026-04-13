## ADDED Requirements

### Requirement: inline output の encoding とバイト境界契約

`stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` はすべてバイト単位で解釈しなければならない（MUST）。`stdout` / `stderr` の文字列値は当該 range 内バイト列を UTF-8 lossy 変換した結果でなければならない（MUST）。

`--max-bytes` の切断がマルチバイト UTF-8 文字の途中を通る場合、該当バイト列は U+FFFD（3 バイト）に置換されなければならない（MUST）。その結果として `stdout` 文字列を UTF-8 エンコードしたバイト長と `stdout_range[1] - stdout_range[0]` の値は一致しない場合がある。

`encoding` フィールドが `"utf-8-lossy"` の場合、文字列内の U+FFFD は元データの非 UTF-8 バイトまたは切断由来の可能性があることをクライアントは想定しなければならない（MUST）。非 lossy な変換を求めるクライアントは `stdout.log` / `stderr.log`（生バイト）を用いる（MUST）。

#### Scenario: max-bytes boundary within multibyte produces U+FFFD

**Given**: a command that outputs the 3-byte UTF-8 sequence for "あ"
**When**: `agent-exec run --max-bytes 2 -- <cmd>` is executed
**Then**: `stdout` contains U+FFFD in place of the truncated character
**And**: `stdout_range[1] - stdout_range[0]` equals 2
**And**: `stdout_total_bytes` equals 3
