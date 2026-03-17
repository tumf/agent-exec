# Design: align-create-run-definition-options

## Overview

The lifecycle split creates two different user entrypoints for job definition:

- `create`: persist a job definition without executing it
- `run`: define a job and start it immediately

To keep the CLI predictable, both commands should share one definition-time option model. The difference between them should be execution timing, not what kind of durable metadata they can express.

## Option Boundary

The proposal formalizes two categories of options.

### Definition-time options

These contribute to persisted job metadata and should be accepted by both `create` and `run`:

- command argv
- cwd
- environment construction inputs
- masking keys for persisted/displayed metadata
- timeout and related execution-definition settings that are stored in `meta.json`
- tags
- completion notification settings
- output-match notification settings
- shell-wrapper inputs used as part of the durable launch definition

### Launch / observation-time options

These control how a caller observes or waits for execution and do not belong on `create`:

- snapshot timing
- tail sizing
- wait behavior
- polling intervals and similar observation controls

## Contract Shape

The simplest durable rule is:

1. `create` accepts all definition-time options and persists them.
2. `run` accepts the same definition-time options and persists them through the same underlying creation path.
3. `run` may additionally accept immediate-start and observation options.
4. `start` consumes the persisted definition rather than redefining it.

This keeps future evolution straightforward: when a new field belongs in `meta.json`, it should be added to both `create` and `run` unless there is an explicit documented reason not to.

## First Concrete Application

This proposal also applies the general rule immediately to the metadata families already under discussion:

- `--tag`
- completion notification options such as `--notify-command` / `--notify-file`
- output-match notification options such as `--output-pattern`, sink selection, match mode, and stream selection

For these fields, the intended contract is:

1. `create` accepts and persists them.
2. `create` does not execute notification sinks or perform output matching.
3. `run` accepts the same definition-time inputs and persists the same metadata shape.
4. `start` activates whatever was saved by `create`.
5. Later changes continue to flow through metadata mutation commands such as `tag set` and `notify set`.

## Relationship to Existing Active Changes

- `add-create-start-lifecycle` defines the lifecycle split and shared primitives.
- `add-job-tags` and `extend-notify-set-output-matches` define specific metadata families that should follow the shared rule.

This proposal absorbs the narrower tags/notifications-only alignment intent into a single broader rule so future definition-time options do not need separate one-off policy proposals.
