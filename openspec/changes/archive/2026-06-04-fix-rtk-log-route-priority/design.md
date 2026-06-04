# Design: RTK Log Route Priority Fix

## Overview

The bug is a routing issue, not a raw observation or compression schema issue. The logs compressor can already deduplicate timestamp-normalized lines when selected explicitly with `--rtk logs`; route mode simply fails to select it for repeated error logs because generic error detection wins.

## Classification Rule

`route` should treat repeated or normalized-repeated log shape as stronger evidence of log output than the presence of the word `ERROR` is evidence of generic error output.

A safe detector can:

- normalize common timestamp prefixes
- normalize changing numeric timestamp components
- count repeated normalized messages
- require a minimum repetition threshold to avoid misclassifying one-off errors

## Preservation Rule

Single, non-repeated errors remain `errors`. This preserves the existing failure-focused behavior for short command failures, stack traces, and one-off diagnostics.

## Verification Shape

The regression fixture should alternate timestamp seconds while keeping the message text constant:

```text
2026-01-01T00:00:00Z ERROR retry failed
2026-01-01T00:00:01Z ERROR retry failed
...
```

Route mode should classify this as `logs`; explicit `--rtk errors` should remain available for users who want error-line extraction.
