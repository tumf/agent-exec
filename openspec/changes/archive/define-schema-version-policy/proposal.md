---
change_type: spec-only
priority: medium
dependencies: []
references:
  - src/schema.rs
  - openspec/specs/agent-exec-contract/spec.md
---

# 変更提案: schema_version のバージョニングポリシーを定義

## Problem / Context

`SCHEMA_VERSION = "0.1"` (`src/schema.rs:18`) は固定値で、bump 基準も CHANGELOG 運用もない。エージェントが version で分岐できず、将来破壊的変更で静かに壊れるリスク。

## Requested Artifact: spec/documentation

policy 定義のみ。

## Proposed Solution

- `agent-exec-contract/spec.md` に以下を MUST で記載:
  - `schema_version` は `"MAJOR.MINOR"` 文字列（Semver 互換）。
  - 後方互換のあるフィールド追加は MINOR bump、既存フィールドの削除・型変更・意味変更は MAJOR bump。
  - bump 時は CHANGELOG.md に `## schema <version>` セクションを追加する（必須）。
  - agent / client は MAJOR 一致を必須、MINOR は新 field を無視して動けなければならない（forward-compat）。

## Acceptance Criteria

- canonical spec に上記ポリシーが記載される。
- 既存 `"0.1"` を canonical version として fixate。
- CHANGELOG.md のひな形セクションを付与（未実施なら Future Work）。

## Out of Scope

- 実際の schema 変更そのもの。
