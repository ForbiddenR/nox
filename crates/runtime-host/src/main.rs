//! `runtime-host` — the supervised Rust sidecar.
//!
//! Speaks JSON-RPC 2.0 over stdio with the Electron main process: length-framed
//! requests in on stdin, responses/notifications out on stdout. stderr is
//! reserved for `tracing` logs captured by the supervisor.

mod dispatch;
mod rpc;

use std::io::{self, BufReader, Write};

use dispatch::{Context, Outcome};
use rpc::{FrameError, PARSE_ERROR, Request, Response};
use serde_json::Value;

fn main() {
    init_tracing();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "runtime-host starting");

    let code = run();
    tracing::info!(code, "runtime-host exiting");
    std::process::exit(code);
}

/// Logs go to stderr only; stdout is the JSON-RPC channel and must stay clean.
fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            EnvFilter::try_from_env("COX_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

/// The blocking read/dispatch/write loop. Returns the process exit code.
fn run() -> i32 {
    // Open the database.
    let db_path = match storage::default_db_path() {
        Ok(path) => path,
        Err(e) => {
            tracing::error!(error = %e, "failed to get database path");
            return 1;
        }
    };

    let db = match storage::Database::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(error = %e, "failed to open database");
            return 1;
        }
    };
    let ctx = Context::new(db);

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    loop {
        let body = match rpc::read_message(&mut reader) {
            Ok(body) => body,
            Err(FrameError::Eof) => {
                tracing::info!("stdin closed; shutting down");
                return 0;
            }
            Err(e) => {
                tracing::error!(error = %e, "framing error; shutting down");
                return 1;
            }
        };

        let req: Request = match serde_json::from_slice(&body) {
            Ok(req) => req,
            Err(e) => {
                tracing::error!(error = %e, "failed to parse request");
                // We couldn't read an id, so reply with null per JSON-RPC.
                let resp = Response::error(Value::Null, PARSE_ERROR, format!("parse error: {e}"));
                if let Err(e) = rpc::write_message(&mut writer, &resp) {
                    tracing::error!(error = %e, "failed to write parse error");
                    return 1;
                }
                continue;
            }
        };

        match dispatch::handle(&ctx, req) {
            Outcome::Reply(resp) => {
                if let Err(e) = rpc::write_message(&mut writer, &resp) {
                    tracing::error!(error = %e, "failed to write response");
                    return 1;
                }
            }
            Outcome::Silent => {}
            Outcome::Shutdown(resp) => {
                let _ = rpc::write_message(&mut writer, &resp);
                let _ = writer.flush();
                return 0;
            }
        }
    }
}
