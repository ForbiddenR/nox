//! Request routing for the sidecar.
//!
//! Routes JSON-RPC requests to their handlers. Lifecycle methods
//! (`health/handshake`, `shutdown`) are handled inline; feature methods
//! delegate to service crates.

use serde_json::{Value, json};
use storage::Database;

use crate::rpc::{METHOD_NOT_FOUND, PROTOCOL_VERSION, Request, Response, INVALID_PARAMS};
use crate::terminal::{TerminalManager, NotificationSender};

const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the read loop should do after handling one message.
pub enum Outcome {
    /// Send this response back to the caller.
    Reply(Response),
    /// Nothing to send (e.g. a notification we don't act on).
    Silent,
    /// Send this final response, then exit the process cleanly.
    Shutdown(Response),
}

/// The dispatch context that holds the database and service state.
pub struct Context {
    db: Database,
    terminal_manager: TerminalManager,
    notification_sender: NotificationSender,
}

impl Context {
    pub fn new(db: Database, notification_sender: NotificationSender) -> Self {
        Self {
            db,
            terminal_manager: TerminalManager::new(),
            notification_sender,
        }
    }
}

/// Route a single decoded request to its handler.
pub fn handle(ctx: &Context, req: Request) -> Outcome {
    // Notifications expect no reply; we have none to act on yet.
    if req.is_notification() {
        tracing::debug!(method = %req.method, "ignoring notification");
        return Outcome::Silent;
    }
    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "health/handshake" => Outcome::Reply(handshake(id, &req.params)),
        "shutdown" => {
            tracing::info!("shutdown requested");
            Outcome::Shutdown(Response::success(id, json!({ "ok": true })))
        }
        "open_project" => Outcome::Reply(open_project(ctx, id, &req.params)),
        "list_recent_projects" => Outcome::Reply(list_recent_projects(ctx, id)),
        "create_thread" => Outcome::Reply(create_thread(ctx, id, &req.params)),
        "list_threads" => Outcome::Reply(list_threads(ctx, id, &req.params)),
        "rename_thread" => Outcome::Reply(rename_thread(ctx, id, &req.params)),
        "archive_thread" => Outcome::Reply(archive_thread(ctx, id, &req.params)),
        "start_run" => Outcome::Reply(start_run(ctx, id, &req.params)),
        "list_runs" => Outcome::Reply(list_runs(ctx, id, &req.params)),
        "list_run_events" => Outcome::Reply(list_run_events(ctx, id, &req.params)),
        "create_terminal" => Outcome::Reply(create_terminal(ctx, id, &req.params)),
        "send_terminal_input" => Outcome::Reply(send_terminal_input(ctx, id, &req.params)),
        "resize_terminal" => Outcome::Reply(resize_terminal(ctx, id, &req.params)),
        "close_terminal" => Outcome::Reply(close_terminal(ctx, id, &req.params)),
        other => {
            tracing::warn!(method = %other, "method not found");
            Outcome::Reply(Response::error(
                id,
                METHOD_NOT_FOUND,
                format!("method not found: {other}"),
            ))
        }
    }
}

/// `health/handshake({ protocol_version })` → `{ runtime_version, protocol_version }`.
/// Refuses on protocol mismatch so the main process can fail fast.
fn handshake(id: Value, params: &Value) -> Response {
    let client_version = params.get("protocol_version").and_then(Value::as_u64);
    match client_version {
        Some(v) if v as u32 == PROTOCOL_VERSION => Response::success(
            id,
            json!({
                "runtime_version": RUNTIME_VERSION,
                "protocol_version": PROTOCOL_VERSION,
            }),
        ),
        Some(v) => Response::error(
            id,
            INVALID_PARAMS,
            format!("protocol version mismatch: client {v}, sidecar {PROTOCOL_VERSION}"),
        ),
        None => Response::error(
            id,
            INVALID_PARAMS,
            "handshake missing protocol_version",
        ),
    }
}

/// `open_project({ path })` → `{ project }`.
fn open_project(ctx: &Context, id: Value, params: &Value) -> Response {
    let path = match params.get("path").and_then(Value::as_str) {
        Some(p) => p,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: path"),
    };

    match project_service::open_project(path, &ctx.db.projects()) {
        Ok(project) => Response::success(id, serde_json::to_value(project).unwrap()),
        Err(e) => Response::error(id, -32000, format!("failed to open project: {e}")),
    }
}

/// `list_recent_projects()` → `{ projects: [...] }`.
fn list_recent_projects(ctx: &Context, id: Value) -> Response {
    match ctx.db.projects().list_recent(20) {
        Ok(projects) => Response::success(
            id,
            json!({ "projects": projects }),
        ),
        Err(e) => Response::error(id, -32000, format!("failed to list projects: {e}")),
    }
}

/// `create_thread({ project_id, title? })` → `{ thread }`.
fn create_thread(ctx: &Context, id: Value, params: &Value) -> Response {
    let project_id_str = match params.get("project_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: project_id"),
    };

    let project_id = match models::Uuid::parse_str(project_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid project_id format"),
    };

    let title = params.get("title").and_then(Value::as_str).map(String::from);

    match thread_service::create_thread(project_id, title, &ctx.db.threads()) {
        Ok(thread) => Response::success(id, serde_json::to_value(thread).unwrap()),
        Err(e) => Response::error(id, -32000, format!("failed to create thread: {e}")),
    }
}

/// `list_threads({ project_id, include_archived? })` → `{ threads: [...] }`.
fn list_threads(ctx: &Context, id: Value, params: &Value) -> Response {
    let project_id_str = match params.get("project_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: project_id"),
    };

    let project_id = match models::Uuid::parse_str(project_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid project_id format"),
    };

    let include_archived = params.get("include_archived").and_then(Value::as_bool).unwrap_or(false);

    match thread_service::list_threads(project_id, include_archived, &ctx.db.threads()) {
        Ok(threads) => Response::success(id, json!({ "threads": threads })),
        Err(e) => Response::error(id, -32000, format!("failed to list threads: {e}")),
    }
}

/// `rename_thread({ thread_id, title })` → `{ thread }`.
fn rename_thread(ctx: &Context, id: Value, params: &Value) -> Response {
    let thread_id_str = match params.get("thread_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: thread_id"),
    };

    let thread_id = match models::Uuid::parse_str(thread_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid thread_id format"),
    };

    let title = match params.get("title").and_then(Value::as_str) {
        Some(t) => t.to_string(),
        None => return Response::error(id, INVALID_PARAMS, "missing required param: title"),
    };

    match thread_service::rename_thread(thread_id, title, &ctx.db.threads()) {
        Ok(thread) => Response::success(id, serde_json::to_value(thread).unwrap()),
        Err(e) => Response::error(id, -32000, format!("failed to rename thread: {e}")),
    }
}

/// `archive_thread({ thread_id })` → `{ thread }`.
fn archive_thread(ctx: &Context, id: Value, params: &Value) -> Response {
    let thread_id_str = match params.get("thread_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: thread_id"),
    };

    let thread_id = match models::Uuid::parse_str(thread_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid thread_id format"),
    };

    match thread_service::archive_thread(thread_id, &ctx.db.threads()) {
        Ok(thread) => Response::success(id, serde_json::to_value(thread).unwrap()),
        Err(e) => Response::error(id, -32000, format!("failed to archive thread: {e}")),
    }
}

/// `create_terminal({ project_id, worktree_id?, cwd? })` → `{ session }`.
fn create_terminal(ctx: &Context, id: Value, params: &Value) -> Response {
    let project_id_str = match params.get("project_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: project_id"),
    };

    let project_id = match models::Uuid::parse_str(project_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid project_id format"),
    };

    let worktree_id = params
        .get("worktree_id")
        .and_then(Value::as_str)
        .and_then(|s| models::Uuid::parse_str(s).ok());

    // Default to project path if no cwd specified
    let cwd = match params.get("cwd").and_then(Value::as_str) {
        Some(path) => path.to_string(),
        None => {
            // Look up project to get its path
            match ctx.db.projects().get(project_id) {
                Ok(Some(project)) => project.path,
                Ok(None) => return Response::error(id, -32000, "project not found"),
                Err(e) => return Response::error(id, -32000, format!("failed to get project: {e}")),
            }
        }
    };

    let session = models::TerminalSession {
        id: models::Uuid::new_v4(),
        project_id,
        worktree_id,
        cwd: cwd.clone(),
        exit_code: None,
        created_at: chrono::Utc::now(),
        closed_at: None,
    };

    // Persist to DB
    if let Err(e) = ctx.db.terminal_sessions().insert(&session) {
        return Response::error(id, -32000, format!("failed to persist session: {e}"));
    }

    // Create PTY session
    match ctx.terminal_manager.create_terminal(session.clone(), ctx.notification_sender.clone()) {
        Ok(_) => Response::success(id, serde_json::to_value(session).unwrap()),
        Err(e) => Response::error(id, -32000, format!("failed to create terminal: {e}")),
    }
}

/// `send_terminal_input({ session_id, data })` → `{ ok: true }`.
fn send_terminal_input(ctx: &Context, id: Value, params: &Value) -> Response {
    let session_id_str = match params.get("session_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: session_id"),
    };

    let session_id = match models::Uuid::parse_str(session_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid session_id format"),
    };

    let data = match params.get("data").and_then(Value::as_str) {
        Some(d) => d,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: data"),
    };

    match ctx.terminal_manager.send_input(session_id, data) {
        Ok(_) => Response::success(id, json!({ "ok": true })),
        Err(e) => Response::error(id, -32000, format!("failed to send input: {e}")),
    }
}

/// `resize_terminal({ session_id, rows, cols })` → `{ ok: true }`.
fn resize_terminal(ctx: &Context, id: Value, params: &Value) -> Response {
    let session_id_str = match params.get("session_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: session_id"),
    };

    let session_id = match models::Uuid::parse_str(session_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid session_id format"),
    };

    let rows = match params.get("rows").and_then(Value::as_u64) {
        Some(r) => r as u16,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: rows"),
    };

    let cols = match params.get("cols").and_then(Value::as_u64) {
        Some(c) => c as u16,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: cols"),
    };

    match ctx.terminal_manager.resize(session_id, rows, cols) {
        Ok(_) => Response::success(id, json!({ "ok": true })),
        Err(e) => Response::error(id, -32000, format!("failed to resize terminal: {e}")),
    }
}

/// `close_terminal({ session_id })` → `{ ok: true }`.
fn close_terminal(ctx: &Context, id: Value, params: &Value) -> Response {
    let session_id_str = match params.get("session_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: session_id"),
    };

    let session_id = match models::Uuid::parse_str(session_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid session_id format"),
    };

    // Close PTY
    if let Err(e) = ctx.terminal_manager.close(session_id) {
        return Response::error(id, -32000, format!("failed to close terminal: {e}"));
    }

    // Update DB
    if let Ok(Some(mut session)) = ctx.db.terminal_sessions().get(session_id) {
        session.closed_at = Some(chrono::Utc::now());
        let _ = ctx.db.terminal_sessions().update(&session);
    }

    Response::success(id, json!({ "ok": true }))
}

/// `start_run({ thread_id, prompt })` → `{ run_id }`.
fn start_run(ctx: &Context, id: Value, params: &Value) -> Response {
    let thread_id_str = match params.get("thread_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: thread_id"),
    };

    let thread_id = match models::Uuid::parse_str(thread_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid thread_id format"),
    };

    let prompt = match params.get("prompt").and_then(Value::as_str) {
        Some(p) => p.to_string(),
        None => return Response::error(id, INVALID_PARAMS, "missing required param: prompt"),
    };

    // Start the run (spawns background task)
    match run_service::start_run(thread_id, prompt, &ctx.db, ctx.notification_sender.clone()) {
        Ok(run_id) => Response::success(id, json!({ "run_id": run_id })),
        Err(e) => Response::error(id, -32000, format!("failed to start run: {e}")),
    }
}

/// `list_runs({ thread_id })` → `{ runs: [...] }`.
fn list_runs(ctx: &Context, id: Value, params: &Value) -> Response {
    let thread_id_str = match params.get("thread_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: thread_id"),
    };

    let thread_id = match models::Uuid::parse_str(thread_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid thread_id format"),
    };

    match ctx.db.list_runs(thread_id) {
        Ok(runs) => Response::success(id, json!({ "runs": runs })),
        Err(e) => Response::error(id, -32000, format!("failed to list runs: {e}")),
    }
}

/// `list_run_events({ run_id })` → `{ events: [...] }`.
fn list_run_events(ctx: &Context, id: Value, params: &Value) -> Response {
    let run_id_str = match params.get("run_id").and_then(Value::as_str) {
        Some(s) => s,
        None => return Response::error(id, INVALID_PARAMS, "missing required param: run_id"),
    };

    let run_id = match models::Uuid::parse_str(run_id_str) {
        Ok(id) => id,
        Err(_) => return Response::error(id, INVALID_PARAMS, "invalid run_id format"),
    };

    match ctx.db.list_run_events(run_id) {
        Ok(events) => Response::success(id, json!({ "events": events })),
        Err(e) => Response::error(id, -32000, format!("failed to list events: {e}")),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::Request;

    fn req(method: &str, id: Option<Value>, params: Value) -> Request {
        Request { jsonrpc: "2.0".into(), id, method: method.into(), params }
    }

    fn test_ctx() -> Context {
        use std::sync::Arc;
        let noop_sender: NotificationSender = Arc::new(|_method, _params| {});
        Context::new(storage::Database::in_memory().unwrap(), noop_sender)
    }

    #[test]
    fn handshake_ok_on_matching_version() {
        let ctx = test_ctx();
        let out = handle(&ctx, req(
            "health/handshake",
            Some(json!(1)),
            json!({ "protocol_version": PROTOCOL_VERSION }),
        ));
        match out {
            Outcome::Reply(r) => {
                assert!(r.error.is_none());
                let result = r.result.unwrap();
                assert_eq!(result["protocol_version"], json!(PROTOCOL_VERSION));
                assert_eq!(result["runtime_version"], json!(RUNTIME_VERSION));
            }
            _ => panic!("expected reply"),
        }
    }

    #[test]
    fn handshake_rejects_mismatch() {
        let ctx = test_ctx();
        let out = handle(&ctx, req(
            "health/handshake",
            Some(json!(1)),
            json!({ "protocol_version": 999 }),
        ));
        match out {
            Outcome::Reply(r) => assert_eq!(r.error.unwrap().code, INVALID_PARAMS),
            _ => panic!("expected reply"),
        }
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let ctx = test_ctx();
        let out = handle(&ctx, req("unknown_method", Some(json!(2)), json!({})));
        match out {
            Outcome::Reply(r) => assert_eq!(r.error.unwrap().code, METHOD_NOT_FOUND),
            _ => panic!("expected reply"),
        }
    }

    #[test]
    fn shutdown_signals_exit() {
        let ctx = test_ctx();
        let out = handle(&ctx, req("shutdown", Some(json!(3)), Value::Null));
        assert!(matches!(out, Outcome::Shutdown(_)));
    }

    #[test]
    fn notification_is_silent() {
        let ctx = test_ctx();
        let out = handle(&ctx, req("run:progress", None, json!({})));
        assert!(matches!(out, Outcome::Silent));
    }

    #[test]
    fn open_project_requires_path() {
        let ctx = test_ctx();
        let out = handle(&ctx, req("open_project", Some(json!(4)), json!({})));
        match out {
            Outcome::Reply(r) => {
                let err = r.error.unwrap();
                assert_eq!(err.code, INVALID_PARAMS);
            }
            _ => panic!("expected reply"),
        }
    }
}
