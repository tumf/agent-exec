# Design: RTK-style JS Python Go Compression

## Overview

This proposal extends command-family-aware compression to common language ecosystems. It should prefer structured parsing when the observed output is already JSON or NDJSON, but it must not alter commands to request structured output.

## Parsing Strategy

- JSON output: parse and group records by rule/code/file/package.
- NDJSON output: parse line by line and aggregate events.
- Plain text: use diagnostic line patterns and state machines.

## Family Priorities

1. Diagnostics that identify actionable failures.
2. Counts and grouping by file/rule/package.
3. Representative examples bounded by cap constants.
4. Omit progress/noise and pass lists.

## Dependency on Test Helpers

This proposal can reuse failure-focused helpers from the Rust/test compression proposal. If implemented before that proposal, it must introduce equivalent helpers without blocking future consolidation.
