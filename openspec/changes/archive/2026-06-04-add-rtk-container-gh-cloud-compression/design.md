# Design: RTK-style Container GitHub Cloud Compression

## Overview

This proposal handles command families where outputs are often tables, JSON, logs, markdown, or transfer progress. It should use local observed text only and must not require credentials or live services for verification.

## Strategies

- Tables: identify headers and keep high-value columns.
- Logs: delegate to generic log compressor.
- Markdown: keep headings, errors, TODOs, check summaries, and bounded body excerpts.
- Cloud JSON: keep resource identity/status/error fields, omit large policies and secrets.
- Progress: strip transfer/progress bars and keep final result context.

## Verification

All behavior should be fixture-backed using synthetic outputs. Real Docker/Kubernetes/GitHub/AWS availability is not required for correctness.
