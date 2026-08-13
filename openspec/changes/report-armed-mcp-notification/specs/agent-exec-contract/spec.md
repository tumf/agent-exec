## MODIFIED Requirements

### Requirement: schema_version のバージョニングポリシー

`schema_version` は `"MAJOR.MINOR"` 形式の文字列でなければならない（MUST）。両セグメントは非負整数であり、先頭ゼロを含んではならない（MUST NOT）。

後方互換のあるフィールド追加（optional field の追加、enum variant の追加）は MINOR bump で行わなければならない（MUST）。既存フィールドの削除、型変更、意味変更、required 化は MAJOR bump を要する（MUST）。

`schema_version` が bump されるとき、repository tracked changelog artifact に対応する `## schema <version>` セクションを追加しなければならない（MUST）。

クライアント／エージェントは MAJOR が一致する JSON を解釈できなければならない（MUST）。未知の optional field を受け取った場合はそれを無視できなければならない（forward compatibility、MUST）。MAJOR 不一致の場合はエラー扱いとしてよい（MAY）。

#### Scenario: MCP run adds optional notification object

**Given**: canonical `schema_version = "0.1"`
**When**: MCP `RunData` gains an optional structured `notification` field
**Then**: the response schema advances to `"0.2"`
**And**: the repository contains a `## schema 0.2` changelog entry describing the optional field
**And**: existing field names, types, and meanings remain unchanged
**And**: clients that ignore unknown optional fields remain compatible
