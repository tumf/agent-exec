//! Implementation of the `serve` sub-command.
//!
//! Starts an HTTP server that exposes job operations as REST endpoints.
//! Endpoints mirror the existing CLI subcommands.
//!
//! Default bind address: `127.0.0.1:19263` (localhost only).
//! Use `--bind 0.0.0.0:19263` to expose externally (requires network access control).

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::jobstore::{JobDir, JobNotFound, resolve_root};
use crate::schema::{
    JobMeta, JobMetaJob, KillData, Response, RunData, SCHEMA_VERSION, StatusData, TailData,
    WaitData,
};

/// Options for the `serve` sub-command.
pub struct ServeOpts {
    pub bind: String,
    pub root: Option<String>,
}

#[derive(Clone)]
struct AppState {
    root: Option<String>,
}

/// Execute `serve`: start the HTTP server and block until shutdown.
pub fn execute(opts: ServeOpts) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main(opts))
}

async fn async_main(opts: ServeOpts) -> Result<()> {
    let state = Arc::new(AppState { root: opts.root });

    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/exec", post(exec_handler))
        .route("/status/{id}", get(status_handler))
        .route("/tail/{id}", get(tail_handler))
        .route("/wait/{id}", get(wait_handler))
        .route("/kill/{id}", post(kill_handler))
        .with_state(state);

    let addr: std::net::SocketAddr = opts
        .bind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address '{}': {e}", opts.bind))?;

    tracing::info!("serve listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

// ---- Shared error/response helpers ----

fn error_json(code: &str, message: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "ok": false,
        "type": "error",
        "error": {
            "code": code,
            "message": message,
            "retryable": false
        }
    })
}

fn err_resp(status: StatusCode, code: &str, message: &str) -> AxumResponse {
    (status, Json(error_json(code, message))).into_response()
}

fn map_err_to_response(e: anyhow::Error) -> AxumResponse {
    if e.downcast_ref::<JobNotFound>().is_some() {
        err_resp(StatusCode::NOT_FOUND, "job_not_found", &format!("{e:#}"))
    } else if e
        .downcast_ref::<crate::jobstore::InvalidJobState>()
        .is_some()
    {
        err_resp(StatusCode::BAD_REQUEST, "invalid_state", &format!("{e:#}"))
    } else {
        err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("{e:#}"),
        )
    }
}

// ---- GET /health ----

async fn health_handler() -> impl IntoResponse {
    let resp = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "ok": true,
        "type": "health"
    });
    (StatusCode::OK, Json(resp))
}

// ---- POST /exec ----

#[derive(Deserialize)]
struct ExecRequest {
    command: Option<Vec<String>>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    timeout_ms: Option<u64>,
    wait: Option<bool>,
}

async fn exec_handler(State(state): State<Arc<AppState>>, request: Request) -> AxumResponse {
    // Read body bytes manually to control error handling.
    let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return err_resp(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "failed to read request body",
            );
        }
    };

    if body_bytes.is_empty() {
        return err_resp(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "request body is required",
        );
    }

    let req: ExecRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return err_resp(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                &format!("invalid JSON: {e}"),
            );
        }
    };

    let command = match req.command {
        Some(c) if !c.is_empty() => c,
        _ => {
            return err_resp(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "command field is required and must be non-empty",
            );
        }
    };

    let root_opt = state.root.clone();
    let env_vars: Vec<String> = req
        .env
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    let cwd = req.cwd;
    let timeout_ms = req.timeout_ms.unwrap_or(0);
    let do_wait = req.wait.unwrap_or(false);

    let result = tokio::task::spawn_blocking(move || {
        run_exec_inner(
            root_opt.as_deref(),
            command,
            cwd.as_deref(),
            env_vars,
            timeout_ms,
            do_wait,
        )
    })
    .await;

    match result {
        Ok(Ok(val)) => (StatusCode::OK, Json(val)).into_response(),
        Ok(Err(e)) => map_err_to_response(e),
        Err(e) => err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("task error: {e}"),
        ),
    }
}

/// Core logic for POST /exec: create and launch a job, return RunData as JSON value.
fn run_exec_inner(
    root: Option<&str>,
    command: Vec<String>,
    cwd: Option<&str>,
    env_vars: Vec<String>,
    timeout_ms: u64,
    do_wait: bool,
) -> Result<serde_json::Value> {
    use crate::run::{
        SnapshotWaitOpts, SpawnSupervisorParams, now_rfc3339_pub, pre_create_log_files,
        resolve_effective_cwd, run_snapshot_wait, spawn_supervisor_process,
    };

    let elapsed_start = std::time::Instant::now();
    let resolved_root = resolve_root(root);
    std::fs::create_dir_all(&resolved_root)
        .map_err(|e| anyhow::anyhow!("create jobs root: {e}"))?;

    let job_id = ulid::Ulid::new().to_string();
    let created_at = now_rfc3339_pub();
    let effective_cwd = resolve_effective_cwd(cwd);
    let shell_wrapper = crate::config::default_shell_wrapper();

    let env_keys: Vec<String> = env_vars
        .iter()
        .map(|kv| kv.split('=').next().unwrap_or(kv).to_string())
        .collect();

    let meta = JobMeta {
        job: JobMetaJob { id: job_id.clone() },
        schema_version: SCHEMA_VERSION.to_string(),
        command: command.clone(),
        created_at,
        root: resolved_root.display().to_string(),
        env_keys,
        env_vars: env_vars.clone(),
        env_vars_runtime: vec![],
        mask: vec![],
        cwd: Some(effective_cwd),
        notification: None,
        inherit_env: true,
        env_files: vec![],
        timeout_ms,
        kill_after_ms: 0,
        progress_every_ms: 0,
        shell_wrapper: Some(shell_wrapper.clone()),
        tags: vec![],
    };

    let job_dir = JobDir::create(&resolved_root, &job_id, &meta)?;
    pre_create_log_files(&job_dir)?;

    spawn_supervisor_process(
        &job_dir,
        SpawnSupervisorParams {
            job_id: job_id.clone(),
            root: resolved_root.clone(),
            full_log_path: job_dir.full_log_path().display().to_string(),
            timeout_ms,
            kill_after_ms: 0,
            cwd: cwd.map(|s| s.to_string()),
            env_vars: env_vars.clone(),
            env_files: vec![],
            inherit_env: true,
            progress_every_ms: 0,
            notify_command: None,
            notify_file: None,
            shell_wrapper: shell_wrapper.clone(),
            command: command.clone(),
        },
    )?;

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    let (final_state, exit_code_opt, finished_at_opt, snapshot, final_snapshot_opt, waited_ms) =
        run_snapshot_wait(
            &job_dir,
            &SnapshotWaitOpts {
                snapshot_after: 0,
                tail_lines: 50,
                max_bytes: 65536,
                wait: do_wait,
                wait_poll_ms: 200,
                wait_until_ms: 0,
                wait_forever: true,
            },
        );

    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    let response = Response::new(
        "run",
        RunData {
            job_id,
            state: final_state,
            tags: vec![],
            env_vars: vec![],
            snapshot,
            stdout_log_path,
            stderr_log_path,
            waited_ms,
            elapsed_ms,
            exit_code: exit_code_opt,
            finished_at: finished_at_opt,
            final_snapshot: final_snapshot_opt,
        },
    );

    Ok(serde_json::to_value(&response)?)
}

// ---- GET /status/:id ----

async fn status_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AxumResponse {
    let root_opt = state.root.clone();
    let result = tokio::task::spawn_blocking(move || {
        let root = resolve_root(root_opt.as_deref());
        let job_dir = JobDir::open(&root, &id)?;
        let meta = job_dir.read_meta()?;
        let st = job_dir.read_state()?;
        let response = Response::new(
            "status",
            StatusData {
                job_id: id,
                state: st.status().as_str().to_string(),
                exit_code: st.exit_code(),
                created_at: meta.created_at,
                started_at: st.started_at().map(|s| s.to_string()),
                finished_at: st.finished_at,
            },
        );
        Ok::<_, anyhow::Error>(serde_json::to_value(&response)?)
    })
    .await;

    match result {
        Ok(Ok(val)) => (StatusCode::OK, Json(val)).into_response(),
        Ok(Err(e)) => map_err_to_response(e),
        Err(e) => err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("task error: {e}"),
        ),
    }
}

// ---- GET /tail/:id ----

async fn tail_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> AxumResponse {
    let root_opt = state.root.clone();
    let result = tokio::task::spawn_blocking(move || {
        let root = resolve_root(root_opt.as_deref());
        let job_dir = JobDir::open(&root, &id)?;
        let stdout_log_path = job_dir.stdout_path();
        let stderr_log_path = job_dir.stderr_path();
        let stdout = job_dir.read_tail_metrics("stdout.log", 50, 65536);
        let stderr = job_dir.read_tail_metrics("stderr.log", 50, 65536);
        let response = Response::new(
            "tail",
            TailData {
                job_id: id,
                stdout_tail: stdout.tail,
                stderr_tail: stderr.tail,
                truncated: stdout.truncated || stderr.truncated,
                encoding: "utf-8-lossy".to_string(),
                stdout_log_path: stdout_log_path.display().to_string(),
                stderr_log_path: stderr_log_path.display().to_string(),
                stdout_observed_bytes: stdout.observed_bytes,
                stderr_observed_bytes: stderr.observed_bytes,
                stdout_included_bytes: stdout.included_bytes,
                stderr_included_bytes: stderr.included_bytes,
            },
        );
        Ok::<_, anyhow::Error>(serde_json::to_value(&response)?)
    })
    .await;

    match result {
        Ok(Ok(val)) => (StatusCode::OK, Json(val)).into_response(),
        Ok(Err(e)) => map_err_to_response(e),
        Err(e) => err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("task error: {e}"),
        ),
    }
}

// ---- GET /wait/:id ----

async fn wait_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> AxumResponse {
    let root_opt = state.root.clone();
    let result = tokio::task::spawn_blocking(move || {
        let root = resolve_root(root_opt.as_deref());
        let job_dir = JobDir::open(&root, &id)?;
        let poll = std::time::Duration::from_millis(200);
        loop {
            let st = job_dir.read_state()?;
            if !st.status().is_non_terminal() {
                let response = Response::new(
                    "wait",
                    WaitData {
                        job_id: id,
                        state: st.status().as_str().to_string(),
                        exit_code: st.exit_code(),
                    },
                );
                return Ok::<_, anyhow::Error>(serde_json::to_value(&response)?);
            }
            std::thread::sleep(poll);
        }
    })
    .await;

    match result {
        Ok(Ok(val)) => (StatusCode::OK, Json(val)).into_response(),
        Ok(Err(e)) => map_err_to_response(e),
        Err(e) => err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("task error: {e}"),
        ),
    }
}

// ---- POST /kill/:id ----

async fn kill_handler(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> AxumResponse {
    let root_opt = state.root.clone();
    let result = tokio::task::spawn_blocking(move || kill_inner(&id, root_opt.as_deref())).await;

    match result {
        Ok(Ok(val)) => (StatusCode::OK, Json(val)).into_response(),
        Ok(Err(e)) => map_err_to_response(e),
        Err(e) => err_resp(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &format!("task error: {e}"),
        ),
    }
}

/// Core logic for POST /kill/:id: send SIGTERM to the job process.
fn kill_inner(job_id: &str, root: Option<&str>) -> Result<serde_json::Value> {
    use crate::schema::JobStatus;

    let resolved_root = resolve_root(root);
    let job_dir = JobDir::open(&resolved_root, job_id)?;
    let st = job_dir.read_state()?;

    let signal = "TERM";

    if *st.status() == JobStatus::Created {
        return Err(anyhow::Error::new(crate::jobstore::InvalidJobState(
            format!(
                "job {} is in 'created' state and has not been started; cannot send signal",
                job_id
            ),
        )));
    }

    if *st.status() == JobStatus::Running
        && let Some(pid) = st.pid
    {
        send_term(pid);
    }
    // If already in a terminal state, it's a no-op (signal ignored gracefully).

    let response = Response::new(
        "kill",
        KillData {
            job_id: job_id.to_string(),
            signal: signal.to_string(),
        },
    );
    Ok(serde_json::to_value(&response)?)
}

/// Send SIGTERM to the process group, falling back to the single process.
#[cfg(unix)]
fn send_term(pid: u32) {
    let pgid = -(pid as libc::pid_t);
    let ret = unsafe { libc::kill(pgid, libc::SIGTERM) };
    if ret != 0 {
        // Fallback: try single-process kill.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    }
}

#[cfg(not(unix))]
fn send_term(_pid: u32) {
    // Windows: not implemented for serve (Windows kill support is in kill.rs).
}
