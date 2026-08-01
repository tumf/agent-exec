//! Embedded managed-job API: linked-consumer integration and end-to-end tests.
//!
//! Every assertion here goes through `agent_exec::embedded` or through a
//! separate fixture consumer binary that links the crate. Nothing in this file
//! spawns the public `agent-exec` CLI or parses a command JSON envelope, so
//! substituting the standalone CLI (or a dummy response) for the typed calls
//! would fail these tests.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use agent_exec::embedded::{
    EmbeddedClient, JobErrorKind, KillRequest, ListRequest, RunRequest, SupervisorDelegation,
    TailRequest,
};

// ---------------------------------------------------------------------------
// Fixture plumbing
// ---------------------------------------------------------------------------

/// The delegating fixture consumer: a separate binary that links the crate and
/// installs the supervisor startup delegation.
fn delegating_consumer() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agent-exec-embedded-consumer"))
}

/// A workload that keeps running until it is signalled.
fn long_running_workload() -> Vec<String> {
    if cfg!(windows) {
        vec!["ping -n 600 127.0.0.1 > NUL".to_string()]
    } else {
        vec!["sleep 600".to_string()]
    }
}

/// A workload that writes into `path`; used to prove a workload did or did not start.
fn sentinel_workload(path: &Path) -> Vec<String> {
    let display = path.display();
    if cfg!(windows) {
        vec![format!("echo started > \"{display}\"")]
    } else {
        vec![format!("printf started > '{display}'")]
    }
}

struct Root {
    _tmp: tempfile::TempDir,
    path: PathBuf,
}

impl Root {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("create temp jobs root");
        let path = tmp.path().to_path_buf();
        Root { _tmp: tmp, path }
    }

    /// A client whose supervisor executable is the delegating fixture consumer.
    fn client(&self) -> EmbeddedClient {
        EmbeddedClient::with_supervisor_exe(&self.path, delegating_consumer())
    }

    /// A client that follows the default constructor and supervises through the
    /// *current* executable.
    ///
    /// The current executable here is this test harness, which never installs
    /// [`agent_exec::embedded::delegate_supervisor_startup`] — so this is a real
    /// consumer that omitted the delegation, not a simulation of one.
    fn client_without_delegation(&self) -> EmbeddedClient {
        EmbeddedClient::new(&self.path).expect("default client")
    }
}

/// Serializes job launches across tests.
///
/// A launch spawns a fresh copy of a large debug binary and must acknowledge
/// within the fixed five-second deadline. Letting every test launch at once
/// makes the suite compete with itself for process startup I/O, which is a
/// property of the test runner rather than of the code under test.
static LAUNCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serialized<T>(launch: impl FnOnce() -> T) -> T {
    let _guard = LAUNCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    launch()
}

/// Create a sibling job directory sharing all but the last character of
/// `job_id`, so prefix resolution becomes genuinely ambiguous.
fn clone_job_dir_with_shared_prefix(root: &Path, job_id: &str) -> String {
    let mut sibling = job_id.to_string();
    let last = sibling.pop().expect("job id is not empty");
    sibling.push(if last == 'a' { 'b' } else { 'a' });

    let src = root.join(job_id);
    let dst = root.join(&sibling);
    std::fs::create_dir_all(&dst).expect("create sibling job dir");
    for name in ["meta.json", "state.json"] {
        let content = std::fs::read_to_string(src.join(name)).expect("read job file");
        std::fs::write(dst.join(name), content.replace(job_id, &sibling)).expect("write job file");
    }
    sibling
}

/// Poll until `predicate` holds or `budget` elapses.
fn wait_until(budget: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + budget;
    loop {
        if predicate() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn field(stdout: &str, key: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.strip_prefix(&format!("{key}="))
            .map(|value| value.trim().to_string())
    })
}

/// Terminate a job so a test never leaves a live workload behind.
fn cleanup(client: &EmbeddedClient, job_id: &str) {
    let _ = client.kill(KillRequest::new(job_id).with_signal("KILL"));
}

// ---------------------------------------------------------------------------
// End-to-end: a separate consumer process manages a job that outlives it
// ---------------------------------------------------------------------------

#[test]
fn separate_consumer_manages_a_job_that_outlives_the_launching_process() {
    let root = Root::new();

    // The launching process is a *different* binary that links the crate.
    let output = serialized(|| {
        Command::new(delegating_consumer())
            .arg(&root.path)
            .args(long_running_workload())
            .env("AGENT_EXEC_FIXTURE_TAGS", "fixture.embedded,recovery")
            .stdin(Stdio::null())
            .output()
            .expect("run fixture consumer")
    });

    let consumer_stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "consumer failed: status={:?} stdout={consumer_stdout} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // The consumer process has now exited. Its stdout carries only the
    // consumer's own lines: no command JSON envelope was written there.
    assert!(
        !consumer_stdout.contains('{') && !consumer_stdout.contains("schema_version"),
        "library calls must not write command JSON to consumer stdout: {consumer_stdout}"
    );
    for line in consumer_stdout.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("job_id=")
                || line.starts_with("state=")
                || line.starts_with("waited_ms="),
            "unexpected consumer stdout line: {line}"
        );
    }

    let job_id = field(&consumer_stdout, "job_id").expect("consumer printed job_id");
    assert_eq!(job_id.len(), 32, "job id should be the full persisted id");
    assert_eq!(
        field(&consumer_stdout, "state").as_deref(),
        Some("running"),
        "bounded inline observation should report the acknowledged running state"
    );

    // A *different* client instance, in this process, observes and controls the
    // same job through the explicit jobs root.
    let client = root.client();

    let status = client.status(&job_id).expect("typed status");
    assert_eq!(status.job_id, job_id);
    assert_eq!(
        status.state, "running",
        "supervision must survive the launching process exit"
    );
    assert!(status.started_at.is_some());

    // Typed tail: real log paths under the explicit root, typed byte metrics.
    let tail = client.tail(TailRequest::new(&job_id)).expect("typed tail");
    assert_eq!(tail.job_id, job_id);
    assert_eq!(tail.encoding, "utf-8-lossy");
    assert!(
        PathBuf::from(&tail.stdout_log_path).starts_with(&root.path),
        "stdout log {} must live under the explicit root {}",
        tail.stdout_log_path,
        root.path.display()
    );
    assert!(PathBuf::from(&tail.stdout_log_path).exists());

    // Typed list: tag-filtered recovery of a job this process never launched.
    let listed = client
        .list(ListRequest::all().with_tags(vec!["fixture.*".to_string()]))
        .expect("typed list");
    let summary = listed
        .jobs
        .iter()
        .find(|job| job.job_id == job_id)
        .unwrap_or_else(|| panic!("tag-filtered list must recover {job_id}: {listed:?}"));
    assert_eq!(summary.state, "running");
    assert_eq!(summary.short_job_id, job_id[..7].to_string());
    assert!(summary.tags.contains(&"recovery".to_string()));

    // A non-matching tag pattern must exclude it (AND semantics preserved).
    let excluded = client
        .list(ListRequest::all().with_tags(vec!["fixture.*".to_string(), "absent.*".to_string()]))
        .expect("typed list with non-matching tag");
    assert!(
        !excluded.jobs.iter().any(|job| job.job_id == job_id),
        "every tag pattern must match"
    );

    // Typed kill: TERM semantics and observed terminal confirmation.
    let killed = client.kill(KillRequest::new(&job_id)).expect("typed kill");
    assert_eq!(killed.job_id, job_id);
    assert_eq!(killed.signal, "TERM");
    // Windows maps every signal onto Job Object termination, so the observed
    // terminal state is platform-dependent; the contract is that kill observes
    // *a* terminal state rather than a specific one.
    assert!(
        matches!(
            killed.state.as_deref(),
            Some("killed" | "exited" | "failed")
        ),
        "kill must observe the terminal state: {killed:?}"
    );
    assert!(killed.observed_within_ms.is_some());

    let terminal = client.status(&job_id).expect("typed status after kill");
    assert_ne!(terminal.state, "running");
    assert!(terminal.finished_at.is_some());
}

#[test]
fn no_wait_launch_returns_an_acknowledged_running_job() {
    let root = Root::new();

    let output = serialized(|| {
        Command::new(delegating_consumer())
            .arg(&root.path)
            .args(long_running_workload())
            .env("AGENT_EXEC_FIXTURE_NO_WAIT", "1")
            .stdin(Stdio::null())
            .output()
            .expect("run fixture consumer")
    });

    let consumer_stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "consumer failed: {consumer_stdout}"
    );

    let job_id = field(&consumer_stdout, "job_id").expect("job_id");
    assert_eq!(
        field(&consumer_stdout, "waited_ms").as_deref(),
        Some("0"),
        "no-wait must skip inline observation"
    );
    assert_eq!(
        field(&consumer_stdout, "state").as_deref(),
        Some("running"),
        "the launch-integrity check still runs for no-wait launches"
    );

    let client = root.client();
    assert_eq!(client.status(&job_id).expect("status").state, "running");
    cleanup(&client, &job_id);
}

// ---------------------------------------------------------------------------
// Negative: launches that never produced validated supervision
// ---------------------------------------------------------------------------

#[test]
fn missing_startup_delegation_fails_the_launch_closed() {
    let root = Root::new();
    let sentinel = root.path.join("workload-ran.txt");
    // Supervision is delegated to this test harness binary, which never
    // installed the delegation entrypoint.
    let client = root.client_without_delegation();

    // Time only the launch itself, not the wait for the shared launch slot.
    let (err, elapsed) = serialized(|| {
        let started = Instant::now();
        let err = client
            .run(RunRequest::new(sentinel_workload(&sentinel)))
            .expect_err("launch without delegation must fail");
        (err, started.elapsed())
    });

    assert_eq!(err.kind(), JobErrorKind::LaunchFailed, "{err}");
    assert!(!err.is_retryable());
    assert!(
        elapsed < Duration::from_secs(8),
        "failure must land within the fixed acknowledgement deadline, took {elapsed:?}"
    );

    // The pre-created job is terminal `failed`, with no `running` transition and
    // no workload launched.
    let jobs = client.list(ListRequest::all()).expect("list");
    assert_eq!(jobs.jobs.len(), 1, "the failed job record is preserved");
    let record = &jobs.jobs[0];
    assert_eq!(record.state, "failed");
    assert!(
        record.started_at.is_none(),
        "no running transition occurred"
    );

    let status = client.status(&record.job_id).expect("status");
    assert_eq!(status.state, "failed");
    assert!(status.started_at.is_none());
    assert!(status.finished_at.is_some());

    assert!(
        !sentinel.exists(),
        "no workload may be launched for a failed launch"
    );
    // No completion notification artifact is produced for a workload that never ran.
    let job_dir = root.path.join(&record.job_id);
    assert!(!job_dir.join("completion_event.json").exists());
}

#[test]
fn unusable_supervisor_executable_fails_the_launch_closed() {
    let root = Root::new();
    let missing = root.path.join("definitely-not-an-executable");
    let client = EmbeddedClient::with_supervisor_exe(&root.path, &missing);

    let err = serialized(|| {
        client
            .run(RunRequest::new(vec!["echo hello".to_string()]))
            .expect_err("unspawnable supervisor must fail the launch")
    });
    assert_eq!(err.kind(), JobErrorKind::LaunchFailed, "{err}");

    let jobs = client.list(ListRequest::all()).expect("list");
    assert_eq!(jobs.jobs.len(), 1);
    assert_eq!(jobs.jobs[0].state, "failed");
    assert!(jobs.jobs[0].started_at.is_none());
}

/// A supervisor executable that neither delegates nor exits must still be
/// bounded by the fixed five-second acknowledgement deadline.
///
/// Unix-only because it needs an executable that hangs regardless of the
/// generated arguments; the remaining launch-failure shapes (early exit,
/// unspawnable executable, late acknowledgement) run on every platform.
#[cfg(unix)]
#[test]
fn supervisor_that_never_acknowledges_fails_on_the_five_second_deadline() {
    use std::os::unix::fs::PermissionsExt;

    let root = Root::new();
    let script = root.path.join("hanging-supervisor.sh");
    std::fs::write(&script, "#!/bin/sh\nexec sleep 600\n").expect("write hanging supervisor");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("chmod hanging supervisor");

    let client = EmbeddedClient::with_supervisor_exe(&root.path, &script);

    // Time only the launch itself, not the wait for the shared launch slot.
    let (err, elapsed) = serialized(|| {
        let started = Instant::now();
        let err = client
            .run(RunRequest::new(vec!["echo hello".to_string()]).no_wait())
            .expect_err("a silent supervisor must fail the launch");
        (err, started.elapsed())
    });

    assert_eq!(err.kind(), JobErrorKind::LaunchFailed, "{err}");
    assert!(
        elapsed >= Duration::from_secs(5),
        "the deadline is fixed at five seconds, failed after {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "the deadline must bound the launch, took {elapsed:?}"
    );

    let jobs = client.list(ListRequest::all()).expect("list");
    assert_eq!(jobs.jobs.len(), 1);
    assert_eq!(
        jobs.jobs[0].state, "failed",
        "no-wait launches perform the same integrity check"
    );
}

#[test]
fn late_supervisor_acknowledgement_cannot_resurrect_a_failed_launch() {
    let root = Root::new();
    let sentinel = root.path.join("late-workload-ran.txt");
    let workload = sentinel_workload(&sentinel);

    // 1. Commit a launch failure (a consumer without startup delegation).
    let failing_client = root.client_without_delegation();
    let err = serialized(|| {
        failing_client
            .run(RunRequest::new(workload.clone()))
            .expect_err("launch must fail")
    });
    assert_eq!(err.kind(), JobErrorKind::LaunchFailed);

    let jobs = failing_client.list(ListRequest::all()).expect("list");
    let job_id = jobs.jobs[0].job_id.clone();
    assert_eq!(jobs.jobs[0].state, "failed");

    // 2. A delegated supervisor arrives late with an otherwise valid reserved
    //    invocation for the same job.
    let late = Command::new(delegating_consumer())
        .arg(agent_exec::embedded::SUPERVISOR_MARKER)
        .arg("--job-id")
        .arg(&job_id)
        .arg("--supervise-root")
        .arg(&root.path)
        .arg("--full-log")
        .arg(root.path.join(&job_id).join("full.log"))
        .arg("--shell-wrapper-resolved")
        .arg(serde_json::to_string(&agent_exec::config::default_shell_wrapper()).expect("json"))
        .arg("--")
        .args(&workload)
        .stdin(Stdio::null())
        .output()
        .expect("run late supervisor");

    assert!(
        !late.status.success(),
        "a late supervisor must fail closed: {}",
        String::from_utf8_lossy(&late.stderr)
    );

    // 3. The terminal state is untouched and the workload never ran.
    let status = failing_client.status(&job_id).expect("status");
    assert_eq!(
        status.state, "failed",
        "a late acknowledgement must not overwrite the terminal state"
    );
    assert!(
        !sentinel.exists(),
        "a late supervisor must not launch the workload"
    );
}

// ---------------------------------------------------------------------------
// Reserved invocation ownership
// ---------------------------------------------------------------------------

#[test]
fn ordinary_consumer_invocations_are_untouched_by_delegation() {
    let root = Root::new();

    // The fixture delegates first and then runs its own argument handling, so a
    // successful launch proves its arguments survived delegation unchanged.
    let output = serialized(|| {
        Command::new(delegating_consumer())
            .arg(&root.path)
            .arg("echo delegation-transparent")
            .stdin(Stdio::null())
            .output()
            .expect("run fixture consumer")
    });

    let consumer_stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "consumer failed: {consumer_stdout}"
    );
    let job_id = field(&consumer_stdout, "job_id").expect("job_id");

    let client = root.client();
    assert!(wait_until(Duration::from_secs(15), || {
        client
            .status(&job_id)
            .map(|status| status.state == "exited")
            .unwrap_or(false)
    }));

    let tail = client.tail(TailRequest::new(&job_id)).expect("tail");
    assert!(
        tail.stdout.contains("delegation-transparent"),
        "the consumer's own arguments must reach the workload: {tail:?}"
    );
}

#[test]
fn in_process_delegation_leaves_ordinary_arguments_alone() {
    // The in-process view of the same guarantee, without spawning anything.
    let outcome = agent_exec::embedded::delegate_supervisor_startup_from([
        "my-consumer",
        "serve",
        "--port",
        "8080",
    ])
    .expect("ordinary arguments must not be claimed");
    assert_eq!(outcome, SupervisorDelegation::NotSupervisor);
}

#[test]
fn malformed_reserved_invocations_fail_closed_in_a_real_consumer() {
    let root = Root::new();
    let job_id = "0123456789abcdef0123456789abcdef";

    // Marker present, generated arguments malformed: the consumer's own command
    // handling must not run, and nothing may be launched.
    let malformed_cases: Vec<Vec<String>> = vec![
        // Missing --supervise-root.
        vec![
            "--job-id".to_string(),
            job_id.to_string(),
            "--".to_string(),
            "echo nope".to_string(),
        ],
        // Unknown flag.
        vec![
            "--job-id".to_string(),
            job_id.to_string(),
            "--supervise-root".to_string(),
            root.path.display().to_string(),
            "--not-a-real-flag".to_string(),
            "--".to_string(),
            "echo nope".to_string(),
        ],
        // Duplicated single-valued flag.
        vec![
            "--job-id".to_string(),
            job_id.to_string(),
            "--job-id".to_string(),
            job_id.to_string(),
            "--supervise-root".to_string(),
            root.path.display().to_string(),
            "--".to_string(),
            "echo nope".to_string(),
        ],
        // Missing the workload separator.
        vec![
            "--job-id".to_string(),
            job_id.to_string(),
            "--supervise-root".to_string(),
            root.path.display().to_string(),
        ],
        // Marker with no generated arguments at all.
        vec![],
    ];

    for case in malformed_cases {
        let output = Command::new(delegating_consumer())
            .arg(agent_exec::embedded::SUPERVISOR_MARKER)
            .args(&case)
            .stdin(Stdio::null())
            .output()
            .expect("run fixture consumer");

        assert!(
            !output.status.success(),
            "malformed reserved invocation must fail closed: {case:?}"
        );
        let stdout = stdout_of(&output);
        assert!(
            !stdout.contains("job_id="),
            "a claimed invocation must never reach consumer command handling: {stdout}"
        );
    }

    // Nothing was created under the root.
    let created = std::fs::read_dir(&root.path)
        .expect("read root")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .count();
    assert_eq!(created, 0, "a failed-closed invocation creates no job");
}

// ---------------------------------------------------------------------------
// Typed surface: errors, filters, and the documented minimal consumer
// ---------------------------------------------------------------------------

#[test]
fn embedded_errors_are_machine_classifiable() {
    let root = Root::new();
    let client = root.client();

    let missing = client
        .status("no-such-job")
        .expect_err("missing job must error");
    assert_eq!(missing.kind(), JobErrorKind::JobNotFound);
    assert!(!missing.is_retryable());

    let missing_tail = client
        .tail(TailRequest::new("no-such-job"))
        .expect_err("missing job must error");
    assert_eq!(missing_tail.kind(), JobErrorKind::JobNotFound);

    let missing_kill = client
        .kill(KillRequest::new("no-such-job"))
        .expect_err("missing job must error");
    assert_eq!(missing_kill.kind(), JobErrorKind::JobNotFound);

    let empty_command = client
        .run(RunRequest::new(vec![]))
        .expect_err("empty command must error");
    assert_eq!(empty_command.kind(), JobErrorKind::InvalidInput);

    let bad_tag = client
        .list(ListRequest::all().with_tags(vec!["**invalid**".to_string()]))
        .expect_err("invalid tag pattern must error");
    assert_eq!(bad_tag.kind(), JobErrorKind::InvalidInput);

    // Ambiguous prefixes are reported with their candidates.
    let first = serialized(|| {
        client
            .run(RunRequest::new(vec!["echo one".to_string()]).no_wait())
            .expect("first launch")
    });
    let sibling = clone_job_dir_with_shared_prefix(&root.path, &first.job_id);
    let shared_prefix = &first.job_id[..first.job_id.len() - 1];

    let ambiguous = client
        .status(shared_prefix)
        .expect_err("a shared prefix must be ambiguous");
    assert_eq!(ambiguous.kind(), JobErrorKind::AmbiguousJobId);
    assert_eq!(ambiguous.candidates().len(), 2, "{ambiguous:?}");
    assert!(ambiguous.candidates().contains(&first.job_id));
    assert!(ambiguous.candidates().contains(&sibling));
    assert!(!ambiguous.is_retryable());

    // The unambiguous full ID still resolves.
    assert_eq!(
        client.status(&first.job_id).expect("exact status").job_id,
        first.job_id
    );

    cleanup(&client, &first.job_id);
}

#[test]
fn typed_list_preserves_limit_truncation_and_state_filtering() {
    let root = Root::new();
    let client = root.client();

    let mut launched = Vec::new();
    for index in 0..3 {
        let run = serialized(|| {
            client
                .run(RunRequest::new(vec![format!("echo job-{index}")]))
                .expect("launch")
        });
        launched.push(run.job_id);
    }

    let all = client.list(ListRequest::all()).expect("list all");
    assert_eq!(all.jobs.len(), 3);
    assert!(!all.truncated);
    assert_eq!(all.skipped, 0);
    assert_eq!(all.root, root.path.display().to_string());

    let limited = client
        .list(ListRequest::all().with_limit(2))
        .expect("limited list");
    assert_eq!(limited.jobs.len(), 2);
    assert!(limited.truncated, "limit must report truncation");

    assert!(wait_until(Duration::from_secs(20), || {
        launched.iter().all(|job_id| {
            client
                .status(job_id)
                .map(|status| status.state == "exited")
                .unwrap_or(false)
        })
    }));

    let exited = client
        .list(ListRequest {
            all: true,
            state: Some("exited".to_string()),
            ..ListRequest::default()
        })
        .expect("state-filtered list");
    assert_eq!(exited.jobs.len(), 3);
    for job in &exited.jobs {
        assert_eq!(job.exit_code, Some(0));
    }

    let none = client
        .list(ListRequest {
            all: true,
            state: Some("killed".to_string()),
            ..ListRequest::default()
        })
        .expect("state-filtered list");
    assert!(none.jobs.is_empty());
}

#[test]
fn typed_run_observes_bounded_initial_output() {
    let root = Root::new();
    let client = root.client();

    let run = serialized(|| {
        client
            .run(RunRequest::new(vec![
                "echo bounded-observation".to_string(),
            ]))
            .expect("launch")
    });

    assert!(wait_until(Duration::from_secs(20), || {
        client
            .status(&run.job_id)
            .map(|status| status.state == "exited")
            .unwrap_or(false)
    }));

    let tail = client.tail(TailRequest::new(&run.job_id)).expect("tail");
    assert!(tail.stdout.contains("bounded-observation"), "{tail:?}");
    assert!(tail.stdout_total_bytes > 0);
    assert_eq!(tail.stdout_range[1], tail.stdout_total_bytes);

    let status = client.status(&run.job_id).expect("status");
    assert_eq!(status.exit_code, Some(0));
}

/// Compile the documented minimal consumer.
///
/// This mirrors the module documentation's startup sequence, so the docs cannot
/// silently drift from the public API.
#[test]
fn documented_minimal_consumer_compiles() {
    fn minimal_consumer(root: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 1. Claim reserved supervisor invocations before consumer arg parsing.
        if agent_exec::embedded::delegate_supervisor_startup()? == SupervisorDelegation::Supervised
        {
            return Ok(String::new());
        }

        // 2. One client, one explicit jobs root.
        let client = EmbeddedClient::new(root)?;

        // 3. Typed operations only.
        let launched = client.run(RunRequest::new(vec!["echo hello".to_string()]))?;
        let status = client.status(&launched.job_id)?;
        let tail = client.tail(TailRequest::new(&launched.job_id))?;
        let jobs = client.list(ListRequest::all())?;
        let killed = client.kill(KillRequest::new(&launched.job_id))?;

        Ok(format!(
            "{} {} {} {} {}",
            status.state,
            tail.encoding,
            jobs.jobs.len(),
            killed.signal,
            launched.stdout_total_bytes
        ))
    }

    // Referencing the function is enough: the point is that it type-checks
    // against the public embedded surface.
    let _ = minimal_consumer;
}

#[test]
fn client_exposes_its_explicit_root_and_supervisor_executable() {
    let root = Root::new();
    let client = root.client();
    assert_eq!(client.root(), root.path.as_path());
    assert_eq!(client.supervisor_exe(), delegating_consumer().as_path());

    // The default constructor selects the current executable, which is what the
    // documented delegation contract requires the consumer to install.
    let default_client = EmbeddedClient::new(&root.path).expect("default client");
    assert_eq!(
        default_client.supervisor_exe(),
        std::env::current_exe().expect("current exe").as_path()
    );
}
