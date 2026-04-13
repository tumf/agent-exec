## ADDED Requirements

### Requirement: Windows state.json の job 識別フィールド

Windows プラットフォーム上で作成される `state.json` は文字列フィールド `windows_job_name` を含まなければならない（MUST）。値の形式は `AgentExec-{job_id}` でなければならない（MUST）。非 Windows プラットフォームでは `windows_job_name` は JSON から省略するか `null` としなければならない（MUST）。

Windows 上の supervisor は、このフィールド値を Job Object 名として用い、子孫プロセス制御に使用しなければならない（MUST）。

#### Scenario: Windows state.json contains windows_job_name

**Given**: `agent-exec run -- cmd /c "exit 0"` is executed on Windows
**When**: `state.json` is written
**Then**: `windows_job_name` equals `AgentExec-{job_id}`

#### Scenario: Unix state.json omits windows_job_name

**Given**: `agent-exec run -- sh -c "exit 0"` is executed on a Unix-like platform
**When**: `state.json` is written
**Then**: `windows_job_name` is absent or `null`
