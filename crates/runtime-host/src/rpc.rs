//! JSON-RPC 2.0 message types and LSP-style `Content-Length` framing.
//!
//! The sidecar reads length-prefixed JSON-RPC requests from stdin and writes
//! responses/notifications to stdout. Length framing (rather than newline
//! delimiting) keeps payloads such as terminal output and diffs unambiguous.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Protocol version exchanged during the handshake. Bump on breaking changes to
/// the command/event contract.
pub const PROTOCOL_VERSION: u32 = 1;

// JSON-RPC 2.0 standard error codes. Some are reserved for handlers landing in
// later milestones.
pub const PARSE_ERROR: i32 = -32700;
#[allow(dead_code)]
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
#[allow(dead_code)]
pub const INTERNAL_ERROR: i32 = -32603;

/// An incoming JSON-RPC request or notification. A notification is simply a
/// request with no `id`.
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    /// True when this is a notification (no `id`, no response expected).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// An outgoing JSON-RPC response (success or error), keyed by request `id`.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

impl Response {
    pub fn success(id: Value, result: Value) -> Self {
        Response { jsonrpc: "2.0", id, result: Some(result), error: None }
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Response {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(ResponseError { code, message: message.into(), data: None }),
        }
    }
}

/// An outgoing JSON-RPC notification (no `id`); used for streaming events.
#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: Value,
}

#[allow(dead_code)]
impl Notification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Notification { jsonrpc: "2.0", method: method.into(), params }
    }
}

/// Errors surfaced by the framing reader.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("end of stream")]
    Eof,
    #[error("malformed header: {0}")]
    Header(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

/// Read one `Content-Length`-framed message body from `reader`.
///
/// Returns the raw JSON bytes of a single message, or [`FrameError::Eof`] when
/// the stream is closed cleanly between messages.
pub fn read_message<R: BufRead>(reader: &mut R) -> Result<Vec<u8>, FrameError> {
    let mut content_length: Option<usize> = None;

    // Read headers until the blank separator line.
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Err(FrameError::Eof);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break; // end of headers
        }
        let (name, value) = trimmed
            .split_once(':')
            .ok_or_else(|| FrameError::Header(trimmed.to_string()))?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            let len = value
                .trim()
                .parse::<usize>()
                .map_err(|_| FrameError::Header(format!("invalid Content-Length: {value}")))?;
            content_length = Some(len);
        }
    }

    let len = content_length
        .ok_or_else(|| FrameError::Header("missing Content-Length".to_string()))?;
    let mut buf = vec![0u8; len];
    io::Read::read_exact(reader, &mut buf)?;
    Ok(buf)
}

/// Serialize and write a `Content-Length`-framed message to `writer`, flushing.
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, message: &T) -> io::Result<()> {
    let body = serde_json::to_vec(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trips_a_message() {
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "health/handshake",
            "params": { "protocol_version": 1 }
        });

        let mut buf = Vec::new();
        write_message(&mut buf, &req).unwrap();

        // Frame should be header + CRLFCRLF + exact body.
        let text = String::from_utf8(buf.clone()).unwrap();
        assert!(text.starts_with("Content-Length: "));
        assert!(text.contains("\r\n\r\n"));

        let mut cursor = Cursor::new(buf);
        let body = read_message(&mut cursor).unwrap();
        let parsed: Request = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.method, "health/handshake");
        assert!(!parsed.is_notification());
    }

    #[test]
    fn reads_two_back_to_back_messages() {
        let mut buf = Vec::new();
        write_message(&mut buf, &serde_json::json!({"jsonrpc":"2.0","id":1,"method":"a"})).unwrap();
        write_message(&mut buf, &serde_json::json!({"jsonrpc":"2.0","method":"b"})).unwrap();

        let mut cursor = Cursor::new(buf);
        let first: Request = serde_json::from_slice(&read_message(&mut cursor).unwrap()).unwrap();
        let second: Request = serde_json::from_slice(&read_message(&mut cursor).unwrap()).unwrap();
        assert_eq!(first.method, "a");
        assert_eq!(second.method, "b");
        assert!(second.is_notification());
    }

    #[test]
    fn clean_eof_between_messages() {
        let mut cursor = Cursor::new(Vec::new());
        assert!(matches!(read_message(&mut cursor), Err(FrameError::Eof)));
    }

    #[test]
    fn case_insensitive_header() {
        let mut buf = Vec::new();
        write!(buf, "content-length: 2\r\n\r\n{{}}").unwrap();
        let mut cursor = Cursor::new(buf);
        let body = read_message(&mut cursor).unwrap();
        assert_eq!(body, b"{}");
    }
}
