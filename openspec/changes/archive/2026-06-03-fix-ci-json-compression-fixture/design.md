# Design: Fix CI JSON Compression Fixture

## Classification

This is an implementation proposal. It changes test coverage for an existing runtime contract and does not require a production behavior change.

## Current Behavior

`json` compression summarizes JSON objects with a shape string such as `object keys=2 [a, b]`. The expansion guard suppresses compressed text when the compressed candidate is greater than or equal to the observed raw output length.

The failing test uses a raw JSON fixture that can be close enough in size to the shape summary that the guard may suppress `compression.stdout`, while the assertion expects the summary to always be present.

## Decision

Keep expansion guard as the source of truth and make the test fixture match the behavior it is trying to verify:

- use a larger JSON object payload to test useful JSON compression
- use a separate short JSON payload to test guard suppression

## Verification Strategy

The implementation must rely on integration tests because the failure appears in the CLI JSON response contract emitted by `agent-exec run`. Unit tests alone would not verify the CLI envelope fields, canonical raw output preservation, or integration with the compression data object.
