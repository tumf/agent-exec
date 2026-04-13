//! Implementation of the `serve` sub-command.
//!
//! Starts an HTTP server that exposes job operations as REST endpoints.
//! Endpoints mirror the existing CLI subcommands.
//!
//! Default bind address: `127.0.0.1:19263` (localhost only).

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use crate::jobstore::{JobDir, JobNotFound, generate_job_id, resolve_root};
use crate::schema::{
    JobMeta, JobMetaJob, KillData, Response, RunData, SCHEMA_VERSION, StatusData, TailData,
    WaitData,
};

/// Options for the `serve` sub-command.
pub struct ServeOpts {
    pub bind: String,
    pub root: Option<String>,
    pub insecure: bool,
    pub allow_origin: Option<String>,
}

#[derive(Clone)]
struct AppState {
    root: Option<String>,
    token: Option<String>,
    allow_origin: Option<String>,
}

/// Returns true if the address is a loopback address.
pub fn is_loopback(addr: &std::net::SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

/// Execute `serve`: start the HTTP server and block until shutdown.
pub fn execute(opts: ServeOpts) -> Result<()> {
    let addr: std::net::SocketAddr = opts
        .bind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address '{}': {e}", opts.bind))?;

    if !is_loopback(&addr) {
        if !opts.insecure {
            let err = error_json(
                "serve_unsafe_bind",
                &format!("refusing to bind to non-loopback address {addr} without --insecure"),
            );
            eprintln!("Error: non-loopback bind address {addr} requires --insecure flag");
            println!("{}", serde_json::to_string(&err).unwrap());
            std::process::exit(1);
        }

        let token = std::env::var("AGENT_EXEC_SERVE_TOKEN").ok();
        if token.as_ref().is_none_or(|t| t.is_empty()) {
            let err = error_json(
                "serve_unsafe_bind",
                &format!(
                    "refusing to bind to non-loopback address {addr} without AGENT_EXEC_SERVE_TOKEN"
                ),
            );
            eprintln!(
                "Error: non-loopback bind address {addr} requires AGENT_EXEC_SERVE_TOKEN to be set"
            );
            println!("{}", serde_json::to_string(&err).unwrap());
            std::process::exit(1);
        }
    }

    if let Some(ref origin) = opts.allow_origin
        && origin == "*"
    {
        let err = error_json("invalid_config", "wildcard '*' origin is not allowed");
        eprintln!("Error: --allow-origin '*' is not permitted");
        println!("{}", serde_json::to_string(&err).unwrap());
        std::process::exit(1);
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main(opts, addr))
}

async fn async_main(opts: ServeOpts, addr: std::net::SocketAddr) -> Result<()> {
    let token = std::env::var("AGENT_EXEC_SERVE_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    let state = Arc::new(AppState {
        root: opts.root,
        token,
        allow_origin: opts.allow_origin,
    });

    let mutating_routes = Router::new()
        .route("/exec", post(exec_handler))
        .route("/kill/{id}", post(kill_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let readonly_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/status/{id}", get(status_handler))
        .route("/tail/{id}", get(tail_handler))
        .route("/wait/{id}", get(wait_handler));

    let mut router = Router::new()
        .merge(mutating_routes)
        .merge(readonly_routes)
        .with_state(state.clone());

    if let Some(ref origin) = state.allow_origin {
        use tower_http::cors::CorsLayer;
        let cors = CorsLayer::new()
            .allow_origin(
                origin
                    .parse::<axum::http::HeaderValue>()
                    .map_err(|e| anyhow::anyhow!("invalid origin '{}': {e}", origin))?,
            )
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ]);
        router = router.layer(cors);
    }

    tracing::info!("serve listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

// ---- Auth middleware ----

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> AxumResponse {
    if let Some(ref expected) = state.token {
        let auth_header = request
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let valid = match auth_header {
            Some(h) if h.starts_with("Bearer ") => &h[7..] == expected.as_str(),
            _ => false,
        };

        if !valid {
            return err_resp(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing or invalid Bearer token",
            );
        }
    }

    next.run(request).await
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
        .downcast_ref::<crate::jobstore::AmbiguousJobId>()
        .is_some()
    {
        err_resp(
            StatusCode::BAD_REQUEST,
            "ambiguous_job_id",
            &format!("{e:#}"),
        )
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
#[serde(deny_unknown_fields)]
struct ExecRequest {
    command: Option<Vec<String>>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
    timeout_ms: Option<u64>,
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

    let result = tokio::task::spawn_blocking(move || {
        run_exec_inner(
            root_opt.as_deref(),
            command,
            cwd.as_deref(),
            env_vars,
            timeout_ms,
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
) -> Result<serde_json::Value> {
    use crate::run::{
        SpawnSupervisorParams, now_rfc3339_pub, observe_inline_output, pre_create_log_files,
        resolve_effective_cwd, spawn_supervisor_process,
    };

    let elapsed_start = std::time::Instant::now();
    let resolved_root = resolve_root(root);
    std::fs::create_dir_all(&resolved_root)
        .map_err(|e| anyhow::anyhow!("create jobs root: {e}"))?;

    let job_id = generate_job_id(&resolved_root)?;
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
        stdin_file: None,
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
            stdin_file: None,
            progress_every_ms: 0,
            notify_command: None,
            notify_file: None,
            shell_wrapper: shell_wrapper.clone(),
            command: command.clone(),
        },
    )?;

    let stdout_log_path = job_dir.stdout_path().display().to_string();
    let stderr_log_path = job_dir.stderr_path().display().to_string();

    let observation = observe_inline_output(&job_dir, true, 10, false, 65536)?;

    let elapsed_ms = elapsed_start.elapsed().as_millis() as u64;

    let response = Response::new(
        "run",
        RunData {
            job_id,
            state: observation.state,
            tags: vec![],
            env_vars: vec![],
            stdout_log_path,
            stderr_log_path,
            elapsed_ms,
            waited_ms: observation.waited_ms,
            stdout: observation.stdout,
            stderr: observation.stderr,
            stdout_range: observation.stdout_range,
            stderr_range: observation.stderr_range,
            stdout_total_bytes: observation.stdout_total_bytes,
            stderr_total_bytes: observation.stderr_total_bytes,
            encoding: observation.encoding,
            exit_code: observation.exit_code,
            finished_at: observation.finished_at,
            signal: observation.signal,
            duration_ms: observation.duration_ms,
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
                job_id: job_dir.job_id.clone(),
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
                job_id: job_dir.job_id.clone(),
                stdout: stdout.tail,
                stderr: stderr.tail,
                encoding: "utf-8-lossy".to_string(),
                stdout_log_path: stdout_log_path.display().to_string(),
                stderr_log_path: stderr_log_path.display().to_string(),
                stdout_range: stdout.range,
                stderr_range: stderr.range,
                stdout_total_bytes: stdout.observed_bytes,
                stderr_total_bytes: stderr.observed_bytes,
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
                        job_id: job_dir.job_id.clone(),
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
            job_id: job_dir.job_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt as _;

    #[test]
    fn test_is_loopback_ipv4_localhost() {
        let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
        assert!(is_loopback(&addr));
    }

    #[test]
    fn test_is_loopback_ipv4_127_range() {
        let addr: std::net::SocketAddr = "127.0.0.2:8080".parse().unwrap();
        assert!(is_loopback(&addr));
    }

    #[test]
    fn test_is_loopback_ipv6() {
        let addr: std::net::SocketAddr = "[::1]:8080".parse().unwrap();
        assert!(is_loopback(&addr));
    }

    #[test]
    fn test_not_loopback_wildcard() {
        let addr: std::net::SocketAddr = "0.0.0.0:8080".parse().unwrap();
        assert!(!is_loopback(&addr));
    }

    #[test]
    fn test_not_loopback_external() {
        let addr: std::net::SocketAddr = "192.168.1.1:8080".parse().unwrap();
        assert!(!is_loopback(&addr));
    }

    #[test]
    fn test_not_loopback_ipv6_all() {
        let addr: std::net::SocketAddr = "[::]:8080".parse().unwrap();
        assert!(!is_loopback(&addr));
    }

    #[test]
    fn test_error_json_structure() {
        let val = error_json("test_code", "test message");
        assert_eq!(val["ok"], false);
        assert_eq!(val["error"]["code"], "test_code");
        assert_eq!(val["error"]["message"], "test message");
        assert_eq!(val["type"], "error");
    }

    fn test_app(token: Option<&str>) -> Router {
        let state = Arc::new(AppState {
            root: None,
            token: token.map(|t| t.to_string()),
            allow_origin: None,
        });
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state)
    }

    fn req(uri: &str, auth: Option<&str>) -> axum::http::Request<axum::body::Body> {
        let mut b = axum::http::Request::builder().uri(uri);
        if let Some(a) = auth {
            b = b.header("Authorization", a);
        }
        b.body(axum::body::Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn test_auth_middleware_no_token_configured() {
        let resp = test_app(None).oneshot(req("/test", None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_token() {
        let resp = test_app(Some("secret123"))
            .oneshot(req("/test", Some("Bearer secret123")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_token() {
        let resp = test_app(Some("secret123"))
            .oneshot(req("/test", None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_wrong_token() {
        let resp = test_app(Some("secret123"))
            .oneshot(req("/test", Some("Bearer wrong")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_non_bearer_scheme() {
        let resp = test_app(Some("secret123"))
            .oneshot(req("/test", Some("Basic secret123")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
