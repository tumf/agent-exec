/// agent-exec v0.1 — core library
///
/// Provides JSON output types, job-directory management, and the
/// implementation of the sub-commands: create, run, start, status, tail,
/// wait, kill, list, schema, install-skills, and completions.
pub mod completions;
pub mod config;
pub mod create;
pub mod delete;
pub mod gc;
pub mod install_skills;
pub mod jobstore;
pub mod kill;
pub mod list;
pub mod notify;
pub mod run;
pub mod schema;
pub mod schema_cmd;
pub mod serve;
pub mod skills;
pub mod start;
pub mod status;
pub mod tag;
pub mod tail;
pub mod wait;
