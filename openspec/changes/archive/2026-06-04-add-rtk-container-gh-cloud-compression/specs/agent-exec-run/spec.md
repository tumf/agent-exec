## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

Container, Kubernetes, GitHub/GitLab CLI, curl/wget, AWS, and table-like outputs routed through `route` compression must use family-specific compact views when recognized (MUST). These views must preserve resource identity, status, failure context, relevant checks, and final result information while pruning low-value columns, progress bars, large nested values, and policy/secret-like content in compressed output (MUST). Compression must not require live credentials or external service access for verification (MUST NOT).

#### Scenario: container and kubernetes tables preserve status

**Given**: observed output from `docker ps`, `docker compose ps`, or `kubectl get pods` contains tabular resource status
**When**: table compression is applied
**Then**: resource names and status/readiness fields are preserved
**And**: abnormal states are prioritized
**And**: less useful columns are omitted or bounded

#### Scenario: gh and glab outputs preserve review state

**Given**: observed output from `gh` or `glab` contains PR, issue, workflow, or check information
**When**: GitHub/GitLab CLI compression is applied
**Then**: identifiers, titles, states, checks, and key labels are preserved
**And**: markdown body text is filtered to relevant bounded sections

#### Scenario: AWS output omits large policy and secret-like content

**Given**: observed AWS CLI output contains JSON or table data with resource metadata and large nested policy or secret-like fields
**When**: AWS compression is applied
**Then**: resource identity, status, and error fields are preserved
**And**: large policy documents and secret-like values are omitted or masked in compressed output

#### Scenario: curl and wget progress is stripped

**Given**: observed curl or wget output includes progress bars or transfer statistics plus a final result or error
**When**: transfer compression is applied
**Then**: progress noise is omitted
**And**: final HTTP/result/error context is preserved
