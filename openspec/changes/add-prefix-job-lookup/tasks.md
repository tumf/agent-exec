## Implementation Tasks

- [x] Task 1: Add `AmbiguousJobId` sentinel error to `src/jobstore.rs` (verification: `cargo build` compiles; unit test asserts Display output includes prefix and candidate list)
- [x] Task 2: Implement prefix-match logic in `JobDir::open` -- exact-match fast path, then directory scan with `starts_with`, returning `AmbiguousJobId` on multiple matches (verification: unit tests in `jobstore::tests` for exact, unique-prefix, ambiguous-prefix, and not-found cases)
- [x] Task 3: Add `AmbiguousJobId` error-handling branch in `src/main.rs` `run()` -- map to `error.code = "ambiguous_job_id"`, `retryable = false`, exit code 1 (verification: `cargo build`; integration test confirms JSON error shape)
- [x] Task 4: Integration test -- prefix lookup succeeds for `status` (verification: `cargo test --test integration prefix_lookup_resolves`)
- [x] Task 5: Integration test -- ambiguous prefix returns `ambiguous_job_id` error (verification: `cargo test --test integration ambiguous_prefix_returns_error`)
- [x] Task 6: Integration test -- cross-command prefix support for at least `tail`, `kill`, `wait` (verification: `cargo test --test integration prefix_lookup_cross_command`)
- [x] Task 7: Run `prek run -a` to confirm fmt, clippy, and all tests pass (verification: exit code 0)

## Future Work

- Add a short-ID display column to `list` output for quick visual reference.
- Consider minimum prefix length policy if the job store grows very large.
