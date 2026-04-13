## ADDED Requirements

### Requirement: schema_version のバージョニングポリシー

`schema_version` は `"MAJOR.MINOR"` 形式の文字列でなければならない（MUST）。両セグメントは非負整数であり、先頭ゼロを含んではならない（MUST NOT）。

後方互換のあるフィールド追加（optional field の追加、enum variant の追加）は MINOR bump で行わなければならない（MUST）。既存フィールドの削除、型変更、意味変更、required 化は MAJOR bump を要する（MUST）。

`schema_version` が bump されるとき、リポジトリ直下の `CHANGELOG.md` に対応する `## schema <version>` セクションを追加しなければならない（MUST）。

クライアント／エージェントは MAJOR が一致する JSON を解釈できなければならない（MUST）。未知の optional field を受け取った場合はそれを無視できなければならない（forward compatibility、MUST）。MAJOR 不一致の場合はエラー扱いとしてよい（MAY）。

#### Scenario: adding an optional field bumps MINOR

**Given**: canonical `schema_version = "0.1"`
**When**: a new optional field is added to `RunData`
**Then**: the next `schema_version` is `"0.2"` with a `## schema 0.2` entry in CHANGELOG.md

#### Scenario: removing a field bumps MAJOR

**Given**: canonical `schema_version = "0.9"`
**When**: an existing field is removed from `RunData`
**Then**: the next `schema_version` is `"1.0"` with a `## schema 1.0` entry in CHANGELOG.md
