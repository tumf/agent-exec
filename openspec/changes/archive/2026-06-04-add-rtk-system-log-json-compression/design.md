# Design: RTK-style System Log JSON Compression

## Overview

System/log/JSON compression must make large generic outputs useful without hiding raw recovery. Since these commands often produce unstructured text, compressors should favor conservative grouping and bounded summaries.

## Strategies

- Listings: directory grouping and omitted counts.
- Search: group by file and match count.
- Text/code: head/tail windows plus structural cues.
- Logs: duplicate grouping, timestamp normalization, error priority.
- JSON: shape summaries, keys, array lengths, and type distribution.
- Env: value masking for secret-like keys.

## Secret Handling

Compression output must not introduce new secret exposure. If an env-like key matches secret patterns, the compressed value should be masked even when raw stdout contains the value. This does not mutate raw canonical fields.
