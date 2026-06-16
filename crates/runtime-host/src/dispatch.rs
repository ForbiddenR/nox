//! Request routing for the sidecar.
//!
//! Routes JSON-RPC requests to their handlers. Lifecycle methods
//! (`health/handshake`, `shutdown`) are handled inline; feature methods
//! delegate to service crates.

use serde_json::{Value, json};
use storage::Database;

use crate::rpc::{METHOD_NOT_FOUND, PROTOCOL_VERSION, Request, Response, INVALID_PARAMS};

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
}

impl Context {
    pub fn new(db: Database) -> Self {
        Self { db }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::Request;

    fn req(method: &str, id: Option<Value>, params: Value) -> Request {
        Request { jsonrpc: "2.0".into(), id, method: method.into(), params }
    }

    fn test_ctx() -> Context {
        Context::new(storage::Database::in_memory().unwrap())
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
