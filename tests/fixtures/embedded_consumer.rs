//! Fixture consumer for the embedded managed-job API.
//!
//! This is a *separate* binary that links `agent-exec` as a library. It never
//! spawns the `agent-exec` executable and never parses a command JSON envelope:
//! every job operation goes through `agent_exec::embedded`.
//! `tests/embedded_consumer.rs` drives it.
//!
//! It doubles as the runnable form of the documented embedding sequence:
//! delegate the reserved supervisor invocation first, then do consumer work.
//!
//! Usage: `agent-exec-embedded-consumer <root> <command...>`
//!
//! Consumer stdout is deliberately minimal (`key=value` lines) so tests can
//! assert that library calls never write command JSON to it.
//!
//! Optional environment knobs, set by the test on this process:
//! * `AGENT_EXEC_FIXTURE_TAGS`    — comma-separated tags for the launched job.
//! * `AGENT_EXEC_FIXTURE_NO_WAIT` — launch without bounded inline observation.

use std::process::ExitCode;

use agent_exec::embedded::{EmbeddedClient, RunRequest, SupervisorDelegation};

fn main() -> ExitCode {
    // Step 1: claim reserved supervisor invocations BEFORE any consumer argument
    // parsing. Ordinary invocations fall straight through untouched.
    match agent_exec::embedded::delegate_supervisor_startup() {
        Ok(SupervisorDelegation::Supervised) => return ExitCode::SUCCESS,
        Ok(SupervisorDelegation::NotSupervisor) => {}
        Err(err) => {
            eprintln!("supervisor delegation failed: {err}");
            return ExitCode::from(3);
        }
    }

    // Step 2: ordinary consumer work, with the consumer's own arguments intact.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: agent-exec-embedded-consumer <root> <command...>");
        return ExitCode::from(2);
    }
    let (root, command) = args.split_first().expect("checked above");

    // Step 3: one client, explicit root, typed calls only.
    let client = match EmbeddedClient::new(root) {
        Ok(client) => client,
        Err(err) => {
            eprintln!("client construction failed: {err}");
            return ExitCode::from(3);
        }
    };

    let tags = std::env::var("AGENT_EXEC_FIXTURE_TAGS")
        .map(|raw| raw.split(',').map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();

    let mut request = RunRequest::new(command.to_vec()).with_tags(tags);
    if std::env::var("AGENT_EXEC_FIXTURE_NO_WAIT").is_ok() {
        request = request.no_wait();
    }

    match client.run(request) {
        Ok(launched) => {
            // Typed data in, typed data out: the consumer picks its own output
            // format and the library writes nothing to this stdout.
            println!("job_id={}", launched.job_id);
            println!("state={}", launched.state);
            println!("waited_ms={}", launched.waited_ms);
            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("error_kind={}", err.kind().as_str());
            println!("error_retryable={}", err.is_retryable());
            eprintln!("run failed: {err}");
            ExitCode::from(4)
        }
    }
}
