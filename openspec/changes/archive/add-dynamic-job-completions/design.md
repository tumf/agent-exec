# Design: add-dynamic-job-completions

## Architecture

### Completion Flow

```
User types: agent-exec status <TAB>
    │
    ▼
Shell invokes completion function (from generated script)
    │
    ▼
clap_complete engine parses partial command line
    │
    ▼
JobIdCompleter::candidates() is called
    │
    ├─ Resolve root directory (--root flag > env > XDG > default)
    ├─ read_dir(root) → list subdirectory names
    ├─ For each dir, optionally read state.json for state
    ├─ Apply context filter (subcommand-dependent)
    └─ Return candidate list with optional descriptions
    │
    ▼
Shell presents candidates to user
```

### Module Structure

```
src/
├── completions.rs    (NEW — JobIdCompleter + helpers)
├── jobstore.rs       (EXISTING — resolve_root reused)
├── main.rs           (MODIFIED — wire completer into <job_id> args)
└── ...
```

### JobIdCompleter

```rust
// Pseudocode — actual API depends on clap_complete version
struct JobIdCompleter {
    /// Optional state filter (None = all jobs)
    state_filter: Option<Vec<&'static str>>,
}

impl ValueCandidates for JobIdCompleter {
    fn candidates(&self) -> Vec<CompletionCandidate> {
        let root = resolve_root_for_completion();
        let entries = match std::fs::read_dir(&root) {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let state = read_job_state(&e.path());
                if let Some(ref filter) = self.state_filter {
                    if !filter.contains(&state.as_str()) {
                        return None;
                    }
                }
                Some(CompletionCandidate::new(name).help(state))
            })
            .collect()
    }
}
```

### Context-Aware Filtering

| Subcommand | Allowed States | Rationale |
|------------|---------------|-----------|
| `start` | `created` | Only un-started jobs can be started |
| `kill` | `running` | Only running jobs can be killed |
| `delete` | `exited`, `killed`, `failed` | Only terminal jobs can be deleted |
| `status` | all | Any job can be inspected |
| `tail` | all | Any job's logs can be viewed |
| `wait` | `created`, `running` | Only non-terminal jobs worth waiting for |
| `tag set` | all | Any job can be tagged |
| `notify set` | all | Any job can have notification configured |

### Error Handling

The completer MUST be resilient:
- Root directory does not exist → empty list
- `state.json` missing or malformed → include job but omit description
- Permission denied on a directory → skip that entry
- No filtering failure should prevent other candidates from being returned

### Performance Considerations

- O(n) directory scan per completion invocation
- O(n) `state.json` reads for descriptions (optional, can be disabled)
- Acceptable for < 1000 jobs (typical agent workload)
- Future optimization: cache results, index by state, or use `gc` to keep
  job count low

## Trade-offs

| Decision | Alternative | Rationale |
|----------|------------|-----------|
| Use `clap_complete` native API | External completion script calling `agent-exec list` | Native API is maintained with clap; external script is fragile and slow |
| Read `state.json` for descriptions | Only list directory names | Small overhead, significantly better UX |
| Per-subcommand state filter | Single completer for all subcommands | Minor code complexity, major UX improvement |
| No caching | LRU cache of directory listing | Premature optimization; completion is invoked infrequently |
