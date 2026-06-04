# Design: RTK-style Rust and Test Compression

## Overview

This proposal focuses on failure-oriented output. The agent needs enough information to diagnose and fix failures, not the full list of passing tests or compile progress.

## Rust Diagnostic Blocks

A diagnostic block starts at lines like:

- `error[E...]`
- `warning: ...`
- `error: ...`

The block may include file locations, code snippets, `note:`, `help:`, and continuation lines. The compressor should keep the first bounded block body and summarize additional similar warnings/errors.

## Test State Machine

For plain text test output, use a small state machine:

- collect test result counts
- track failed test names
- collect failure blocks
- bound stack/backtrace lines
- emit summary plus failure details

## Safety

If the parser cannot confidently identify a test or diagnostic structure, fall back to generic errors/summary behavior and let expansion guard suppress non-useful candidates.
