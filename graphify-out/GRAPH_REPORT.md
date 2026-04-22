# Graph Report - openspec/specs  (2026-04-22)

## Corpus Check
- Corpus is ~8,503 words - fits in a single context window. You may not need a graph.

## Summary
- 34 nodes · 48 edges · 6 communities detected
- Extraction: 85% EXTRACTED · 15% INFERRED · 0% AMBIGUOUS · INFERRED: 7 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_JSON Contract|JSON Contract]]
- [[_COMMUNITY_Run Serve IO|Run Serve I/O]]
- [[_COMMUNITY_Jobstore Windows|Jobstore Windows]]
- [[_COMMUNITY_Core Harness|Core Harness]]
- [[_COMMUNITY_Run Logging|Run Logging]]
- [[_COMMUNITY_Skills Packaging|Skills Packaging]]

## God Nodes (most connected - your core abstractions)
1. `Inline Output Contract` - 6 edges
2. `agent-exec Canonical Spec` - 5 edges
3. `agent-exec Jobstore Spec` - 5 edges
4. `agent-exec Serve Spec` - 5 edges
5. `JSON-only stdout` - 5 edges
6. `Common JSON Envelope` - 5 edges
7. `Serve REST API` - 4 edges
8. `Integration Test Contract` - 4 edges
9. `agent-exec Contract Spec` - 3 edges
10. `Tail Range Contract` - 3 edges

## Surprising Connections (you probably didn't know these)
- `JSON-only stdout` --semantically_similar_to--> `Common JSON Envelope`  [INFERRED] [semantically similar]
  openspec/specs/agent-exec/spec.md → openspec/specs/agent-exec-contract/spec.md
- `AGENT_EXEC_ROOT Test Isolation` --conceptually_related_to--> `Integration Test Contract`  [INFERRED]
  openspec/specs/agent-exec-test-harness/spec.md → openspec/specs/agent-exec-tests/spec.md
- `agent-exec JSON Printing Spec` --references--> `JSON-only stdout`  [EXTRACTED]
  openspec/specs/agent-exec-json-printing/spec.md → openspec/specs/agent-exec/spec.md
- `agent-exec Run Spec` --references--> `Inline Output Contract`  [EXTRACTED]
  openspec/specs/agent-exec-run/spec.md → openspec/specs/agent-exec/spec.md
- `agent-exec Jobstore Spec` --references--> `Hash-like Job ID`  [EXTRACTED]
  openspec/specs/agent-exec-jobstore/spec.md → openspec/specs/agent-exec/spec.md

## Hyperedges (group relationships)
- **Job Lifecycle Contract** — inline_output_contract, job_directory_structure, state_json_schema [INFERRED 0.84]
- **Serve HTTP Surface** — serve_rest_api, inline_output_contract, common_json_envelope [EXTRACTED 1.00]

## Communities

### Community 0 - "JSON Contract"
Cohesion: 0.43
Nodes (7): agent-exec Contract Spec, agent-exec JSON Printing Spec, agent-exec Tests Spec, Common JSON Envelope, Error Object Format, Integration Test Contract, JSON-only stdout

### Community 1 - "Run Serve I/O"
Cohesion: 0.48
Nodes (7): agent-exec Run Spec, agent-exec Serve Spec, Inline Output Contract, Rationale: Distinguish True Workload Termination, Serve Auth Guard, Serve REST API, Tail Range Contract

### Community 2 - "Jobstore Windows"
Cohesion: 0.43
Nodes (7): agent-exec Jobstore Spec, agent-exec Windows Spec, Job Directory Structure, meta.json Schema, state.json Schema, windows_job_name Field, Windows Job Object Management

### Community 3 - "Core Harness"
Cohesion: 0.4
Nodes (6): agent-exec Canonical Spec, agent-exec Test Harness Spec, AGENT_EXEC_ROOT Test Isolation, Hash-like Job ID, Jobstore Root Precedence, Prefix Job Lookup

### Community 4 - "Run Logging"
Cohesion: 0.67
Nodes (4): agent-exec Run Logging Spec, full.log Human View, Rationale: full.log Is Not for Machine Parsing, Stream Logs as Machine Source

### Community 5 - "Skills Packaging"
Cohesion: 1.0
Nodes (3): agent-exec Skills Spec, Embedded Skill Installation, Skill Lock File

## Knowledge Gaps
- **4 isolated node(s):** `agent-exec JSON Printing Spec`, `agent-exec Tests Spec`, `agent-exec Test Harness Spec`, `Rationale: Distinguish True Workload Termination`
  These have ≤1 connection - possible missing edges or undocumented components.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `agent-exec Jobstore Spec` connect `Jobstore Windows` to `Core Harness`?**
  _High betweenness centrality (0.234) - this node is a cross-community bridge._
- **Why does `agent-exec Canonical Spec` connect `Core Harness` to `JSON Contract`, `Run Serve I/O`?**
  _High betweenness centrality (0.227) - this node is a cross-community bridge._
- **Why does `Jobstore Root Precedence` connect `Core Harness` to `Jobstore Windows`?**
  _High betweenness centrality (0.154) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `JSON-only stdout` (e.g. with `Common JSON Envelope` and `Integration Test Contract`) actually correct?**
  _`JSON-only stdout` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `agent-exec JSON Printing Spec`, `agent-exec Tests Spec`, `agent-exec Test Harness Spec` to the rest of the system?**
  _4 weakly-connected nodes found - possible documentation gaps or missing edges._
