# Design: RTK-style Compression Routing Foundation

## Overview

The foundation keeps `agent-exec` as a job observation tool rather than a command proxy. Unlike original RTK, it does not inject flags such as JSON output modes or rewrite commands. It observes the output produced by the user-supplied command and produces a compact side-channel view in the `compression` object.

## Architecture

Recommended layout:

```text
src/compress/
  mod.rs
  route.rs
  util.rs
  generic.rs
  git.rs
  rust.rs
  tests.rs
  js.rs
  python.rs
  go.rs
  system.rs
  logs.rs
  json.rs
  containers.rs
  gh.rs
  cloud.rs
```

`mod.rs` should preserve the public API currently consumed by command implementations:

- `CompressionMode`
- `CompressionInput`
- `resolve_cli_mode`
- `compress`

## Routing Model

Routing should be deterministic and local:

1. If mode is explicit and not `route`, map the mode to a matching compressor family.
2. If mode is `route`, inspect `command` first.
3. Use output-shape detection as a fallback.
4. If no specific family matches, use generic summary/log/json behavior.

Command inspection must handle command arrays produced by direct commands and shell wrappers conservatively. For shell strings, detection may use token containment only when it is safe and tested.

## Compatibility

Raw fields remain canonical. The compressed view never replaces:

- `stdout`
- `stderr`
- `stdout_range`
- `stderr_range`
- `stdout_total_bytes`
- `stderr_total_bytes`
- log path fields

## Expansion Guard

All specialized compressors must return candidates to a shared guard. The guard decides whether `compression.applied` is true and prevents larger/equal compressed text from being embedded.

## Dependency Relationships

This foundation is required before specialized proposals can add broad command families without reworking the public compression API. Dependent proposals can be implemented in parallel after this foundation lands if their modules use separate files and shared helpers.
