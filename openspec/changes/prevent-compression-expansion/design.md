# Design: Compression Expansion Guard

## Overview

Compression is an optional compact view layered on top of raw observations. It must be net helpful. If generated compressed text is not smaller than the raw observed stream, the response should not pay the extra token cost of including that text.

## Requested Artifact

Implementation.

## Request Normalization

User-facing outcomes:

- `tail` continues to tail command output.
- Compression stays enabled by default through `route`.
- Compression must not make responses materially larger by including expanded compressed payloads.
- Raw observation fields remain the source of truth.

Repository areas likely requiring change:

- `src/compress.rs` for guard logic.
- `src/schema.rs` only if an explicit reason field is needed beyond existing strategy fields.
- `tests/integration.rs` for regression tests.

## Guard Rule

For each stream, compare generated compressed text bytes with the raw observed stream text bytes available to the compressor.

Recommended condition:

```text
if compressed_stdout.len() >= raw_stdout.len()
  and compressed_stderr.len() >= raw_stderr.len()
  and no other stream had useful reduction:
    return applied=false, compact metadata only
```

A stream with empty raw text should not be treated as successfully compressed merely because compressed text is also empty.

The fallback payload must be bounded. It should not echo raw or compressed text. It can use existing fields:

```json
{
  "compression": {
    "mode": "route",
    "applied": false,
    "detected_kind": "json",
    "stdout": "",
    "stderr": "",
    "stdout_original_bytes": 6091,
    "stderr_original_bytes": 127,
    "stdout_compressed_bytes": 0,
    "stderr_compressed_bytes": 0,
    "omitted": false,
    "strategy": ["expansion-guard"]
  }
}
```

If a schema change is preferred, a future field such as `reason: "would_expand"` is acceptable, but it is not required if `strategy` can carry the reason.

## Per-Stream vs Whole-Payload Trade-off

The simpler initial rule is whole-payload suppression: if the generated compressed view would not reduce the combined `stdout + stderr` observed text, suppress both compressed streams and set `applied=false`.

This avoids complex partial semantics and is easier to test. Partial per-stream preservation can be considered later if users need it.

## Verification Strategy

Use integration tests because this is a CLI response contract.

Required regression shape:

- Launch a job that emits JSON/NDJSON-like command output.
- Call `tail` or `run` with default `route` compression.
- Assert `compression.applied=false` and `strategy` contains `expansion-guard` when the compressed candidate would be larger.

Required non-regression shape:

- Launch a job with repeated log lines where logs compression reduces output.
- Assert `compression.applied=true` and compact repeated-line text is present.

## Non-Goals

This change does not revisit whether `tail` should compress by default. It only ensures default compression does not add a large expanded compressed view.
