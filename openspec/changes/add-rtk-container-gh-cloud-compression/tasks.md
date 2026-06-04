## Implementation Tasks

- [ ] Add route detection for `docker`, `docker compose`, `kubectl`, `gh`, `glab`, `aws`, `curl`, `wget`, and psql-like table commands (verification: unit - classifier maps representative argv arrays to expected detected kinds).
- [ ] Implement Docker and Docker Compose table compression preserving container/service name, status, ports when useful, and unhealthy/exited states (verification: unit - docker ps/images/compose ps fixtures compact to key columns and highlight abnormal states).
- [ ] Implement kubectl table compression preserving resource name, namespace when present, ready/status/restarts/age, and abnormal conditions (verification: unit - pod/service/deployment table fixtures compact with failures prioritized).
- [ ] Reuse log compression for `docker logs` and `kubectl logs` routed outputs (verification: integration - repeated container log fixture deduplicates and preserves error lines).
- [ ] Implement `gh`/`glab` output compression for PR/issue/list/view/run/check-like outputs, including markdown body filtering and status/check summaries (verification: unit - gh/glab fixtures retain number/title/state/checks and bound body text).
- [ ] Implement AWS output compression for JSON/table outputs, preserving identity/resource/status/error fields and omitting policy documents, secrets, and large nested values (verification: unit - AWS fixtures compact STS, EC2 list, Lambda list, CloudFormation events, and logs outputs).
- [ ] Implement curl/wget progress filtering and result/error summaries (verification: unit - progress fixture strips transfer bars while preserving final HTTP/error context).
- [ ] Ensure all compressors avoid external network credentials and use expansion guard (verification: integration - fixture-backed synthetic commands produce guarded or smaller compression output with raw fields intact).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Real cloud account or cluster verification is intentionally excluded; local fixtures and mocked command output are authoritative for this proposal.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-container-gh-cloud-compression --archive-gate`
