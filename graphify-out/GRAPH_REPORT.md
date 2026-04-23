# Graph Report - agent-exec  (2026-04-24)

## Corpus Check
- 25 files · ~127,828 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 690 nodes · 2197 edges · 29 communities detected
- Extraction: 78% EXTRACTED · 22% INFERRED · 0% AMBIGUOUS · INFERRED: 480 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]

## God Nodes (most connected - your core abstractions)
1. `assert_envelope()` - 145 edges
2. `binary()` - 33 edges
3. `supervise()` - 28 edges
4. `wait_until_terminal()` - 24 edges
5. `resolve_root()` - 24 edges
6. `list_job_candidates()` - 22 edges
7. `JobDir` - 21 edges
8. `execute()` - 21 edges
9. `execute()` - 19 edges
10. `execute()` - 16 edges

## Surprising Connections (you probably didn't know these)
- `set()` --calls--> `resolve_root()`  [INFERRED]
  src/notify.rs → src/jobstore.rs
- `run()` --calls--> `set()`  [INFERRED]
  src/main.rs → src/notify.rs
- `execute()` --calls--> `build_output_match_config()`  [INFERRED]
  src/create.rs → src/notify.rs
- `execute()` --calls--> `build_output_match_config()`  [INFERRED]
  src/run.rs → src/notify.rs
- `run_exec_inner()` --calls--> `resolve_root()`  [INFERRED]
  src/serve.rs → src/jobstore.rs

## Hyperedges (group relationships)
- **Job Lifecycle Contract** — inline_output_contract, job_directory_structure, state_json_schema [INFERRED 0.84]
- **Serve HTTP Surface** — serve_rest_api, inline_output_contract, common_json_envelope [EXTRACTED 1.00]

## Communities

### Community 0 - "Community 0"
Cohesion: 0.06
Nodes (90): argv_mode_completion_aligns_with_workload_boundary_issue5_regression(), argv_mode_exec_handoff_completes(), argv_mode_non_unix_shell_string_fallback_completes(), assert_envelope(), create_does_not_trigger_notification_side_effects(), create_start_reuses_stdin_definition(), delete_dry_run_single_preserves_directory(), delete_nonexistent_job_returns_job_not_found() (+82 more)

### Community 1 - "Community 1"
Cohesion: 0.07
Nodes (61): ambiguous_prefix_returns_error(), binary(), completions_invalid_shell_exits_with_code_2(), create_no_tags_persists_empty_array(), create_notify_command_persisted_same_shape_as_run(), create_output_pattern_persisted_same_shape_as_run(), create_tag_deduplication(), create_tag_persisted_same_shape_as_run() (+53 more)

### Community 2 - "Community 2"
Cohesion: 0.09
Nodes (38): main(), CreateOpts, execute(), Shell, assign_to_job_object(), dispatch_command_sink(), dispatch_file_sink(), execute() (+30 more)

### Community 3 - "Community 3"
Cohesion: 0.04
Nodes (43): CompletionEvent, CompletionEventRecord, CreateData, DeleteData, DeleteJobResult, error_detail_includes_details_when_present(), error_detail_omits_details_when_none(), ErrorDetail (+35 more)

### Community 4 - "Community 4"
Cohesion: 0.08
Nodes (44): assert_usage_error(), create_with_stdin_dash_materializes_input_for_later_start(), global_root_flag_gc(), global_root_flag_list(), global_root_flag_run(), global_root_flag_status(), list_exact_tag_filter(), list_invalid_tag_pattern_rejected() (+36 more)

### Community 5 - "Community 5"
Cohesion: 0.18
Nodes (33): assert_common_fields(), binary(), free_port(), get_json(), options_request(), parse_curl_output(), post_json(), post_json_with_auth() (+25 more)

### Community 6 - "Community 6"
Cohesion: 0.12
Nodes (24): AmbiguousJobId, generate_job_id(), generate_job_id_fails_after_16_collisions(), generate_job_id_retries_when_collision_exists(), generate_job_id_returns_fixed_length_hex(), generate_job_id_with_rng(), HeadMetrics, init_state_writes_deterministic_job_name_on_windows() (+16 more)

### Community 7 - "Community 7"
Cohesion: 0.16
Nodes (27): complete_all_jobs(), complete_created_jobs(), complete_running_jobs(), complete_terminal_jobs(), complete_waitable_jobs(), extract_root_from_argv(), extract_root_from_comp_line(), extract_root_from_line() (+19 more)

### Community 8 - "Community 8"
Cohesion: 0.11
Nodes (17): AppState, async_main(), error_json(), ExecParams, ExecRequest, execute(), is_loopback(), KillQuery (+9 more)

### Community 9 - "Community 9"
Cohesion: 0.09
Nodes (19): Cli, Command, CompletionShell, main(), normalize_wait_flags(), NotifySubcommand, parse_filter_pattern(), parse_stored_tag() (+11 more)

### Community 10 - "Community 10"
Cohesion: 0.12
Nodes (27): agent-exec Contract Spec, agent-exec Jobstore Spec, agent-exec JSON Printing Spec, agent-exec Run Spec, agent-exec Serve Spec, agent-exec Canonical Spec, agent-exec Test Harness Spec, agent-exec Tests Spec (+19 more)

### Community 11 - "Community 11"
Cohesion: 0.13
Nodes (16): execute(), execute_inner(), KillOpts, observe_post_signal(), PostSignalObservation, send_signal(), send_signal_no_job(), terminate_process_tree() (+8 more)

### Community 12 - "Community 12"
Cohesion: 0.17
Nodes (17): resolve_root(), resolve_root_cli_flag_wins(), resolve_root_default_contains_agent_exec(), resolve_root_env_var(), resolve_root_xdg(), execute(), schema_file_path(), SchemaOpts (+9 more)

### Community 13 - "Community 13"
Cohesion: 0.13
Nodes (8): dir_size_bytes(), dir_size_bytes_with_file(), execute(), format_rfc3339(), GcOpts, is_leap(), is_older_than(), parse_duration()

### Community 14 - "Community 14"
Cohesion: 0.14
Nodes (11): delete_all(), delete_single(), DeleteOpts, execute(), execute(), InstallSkillsOpts, InvalidJobState, short_job_id() (+3 more)

### Community 15 - "Community 15"
Cohesion: 0.18
Nodes (15): AgentExecConfig, default_shell_wrapper(), default_wrapper_is_nonempty(), discover_config_path(), load_config(), load_config_parses_unix_wrapper(), parse_shell_wrapper_str(), parse_shell_wrapper_str_splits_whitespace() (+7 more)

### Community 16 - "Community 16"
Cohesion: 0.24
Nodes (3): JobDir, JobNotFound, state_json_contains_updated_at()

### Community 17 - "Community 17"
Cohesion: 0.15
Nodes (13): delete_all_deleted_action_implies_directories_absent(), delete_all_distinguishes_out_of_scope_from_in_scope_skipped(), delete_all_dry_run_preserves_directories(), delete_all_response_includes_cwd_scope(), delete_all_scopes_to_current_cwd(), delete_all_skips_running_and_created_jobs(), delete_all_skips_terminal_state_with_live_pid(), list_all_flag_disables_cwd_filter() (+5 more)

### Community 18 - "Community 18"
Cohesion: 0.21
Nodes (9): days_to_ymd(), EmbeddedFile, install_builtin(), InstalledSkill, is_leap(), LockEntry, LockFile, now_rfc3339() (+1 more)

### Community 19 - "Community 19"
Cohesion: 0.27
Nodes (10): assert_gc_envelope(), gc_custom_older_than_flag_reported(), gc_deleted_action_implies_directory_absent_and_categorises_skips(), gc_deletes_only_terminal_jobs(), gc_dry_run_preserves_directories(), gc_empty_root_returns_ok(), gc_skips_jobs_without_gc_timestamp(), gc_skips_unreadable_state() (+2 more)

### Community 20 - "Community 20"
Cohesion: 0.32
Nodes (8): get_dynamic_candidates(), get_dynamic_candidates_via_root_arg(), test_dynamic_completion_all_jobs_for_status(), test_dynamic_completion_empty_when_root_missing(), test_dynamic_completion_excludes_jobs_from_other_cwd(), test_dynamic_completion_running_only_for_kill(), test_dynamic_completion_with_root_arg_returns_jobs_from_that_path(), write_completion_job()

### Community 21 - "Community 21"
Cohesion: 0.33
Nodes (6): run_completion(), test_completions_bash_outputs_nonempty_script(), test_completions_fish_outputs_nonempty_script(), test_completions_invalid_shell_exits_with_code_2(), test_completions_powershell_outputs_nonempty_script(), test_completions_zsh_outputs_nonempty_script()

### Community 22 - "Community 22"
Cohesion: 0.4
Nodes (4): KillOpts<'a>, build_output_match_config(), NotifySetOpts, set()

### Community 23 - "Community 23"
Cohesion: 0.4
Nodes (5): completions_bash_outputs_nonempty_script(), completions_fish_outputs_nonempty_script(), completions_powershell_outputs_nonempty_script(), completions_zsh_outputs_nonempty_script(), run_raw()

### Community 24 - "Community 24"
Cohesion: 0.5
Nodes (1): FixedRng

### Community 25 - "Community 25"
Cohesion: 0.67
Nodes (4): agent-exec Run Logging Spec, full.log Human View, Rationale: full.log Is Not for Machine Parsing, Stream Logs as Machine Source

### Community 26 - "Community 26"
Cohesion: 1.0
Nodes (3): agent-exec Skills Spec, Embedded Skill Installation, Skill Lock File

### Community 27 - "Community 27"
Cohesion: 1.0
Nodes (1): WaitOpts<'a>

### Community 28 - "Community 28"
Cohesion: 1.0
Nodes (1): TailOpts<'a>

## Knowledge Gaps
- **78 isolated node(s):** `NotifySetOpts`, `ServeOpts`, `AppState`, `ExecRequest`, `ExecParams` (+73 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 24`** (5 nodes): `FixedRng`, `.fill_bytes()`, `.next_u32()`, `.next_u64()`, `.try_fill_bytes()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 27`** (2 nodes): `WaitOpts<'a>`, `.default()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 28`** (2 nodes): `TailOpts<'a>`, `.default()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `JobStatus` connect `Community 11` to `Community 0`, `Community 3`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **Why does `supervise()` connect `Community 2` to `Community 0`, `Community 6`, `Community 7`, `Community 9`, `Community 14`, `Community 16`?**
  _High betweenness centrality (0.056) - this node is a cross-community bridge._
- **Why does `execute()` connect `Community 13` to `Community 0`, `Community 11`, `Community 12`, `Community 14`?**
  _High betweenness centrality (0.050) - this node is a cross-community bridge._
- **Are the 16 inferred relationships involving `supervise()` (e.g. with `.open()` and `.read_meta()`) actually correct?**
  _`supervise()` has 16 INFERRED edges - model-reasoned connections that need verification._
- **Are the 19 inferred relationships involving `resolve_root()` (e.g. with `set()` and `run_exec_inner()`) actually correct?**
  _`resolve_root()` has 19 INFERRED edges - model-reasoned connections that need verification._
- **What connects `NotifySetOpts`, `ServeOpts`, `AppState` to the rest of the system?**
  _78 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.06 - nodes in this community are weakly interconnected._
