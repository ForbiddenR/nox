//! Terminal service: manage PTY-backed terminal sessions.

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Errors from terminal operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("terminal session not found: {0}")]
    NotFound(Uuid),
    #[error("pty error: {0}")]
    Pty(#[from] anyhow::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Terminal output or exit event.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalEvent {
    Output { session_id: Uuid, data: String },
    Exit { session_id: Uuid, exit_code: Option<u32> },
}

/// A managed terminal session with PTY.
struct Session {
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}

/// Terminal service manages all active PTY sessions.
pub struct TerminalService {
    sessions: Arc<Mutex<HashMap<Uuid, Session>>>,
}

impl TerminalService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new terminal session in the given working directory.
    /// Returns (session_id, event_receiver).
    pub fn create_terminal(
        &self,
        cwd: impl AsRef<Path>,
    ) -> Result<(Uuid, mpsc::UnboundedReceiver<TerminalEvent>), Error> {
        let session_id = Uuid::new_v4();
        let pty_system = native_pty_system();

        // Create PTY with default size
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Pty(e.into()))?;

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.cwd(cwd.as_ref());

        let child = pair.slave.spawn_command(cmd).map_err(|e| Error::Pty(e.into()))?;

        let master = Arc::new(Mutex::new(pair.master));
        let child = Arc::new(Mutex::new(child));

        let session = Session {
            master: master.clone(),
            child: child.clone(),
        };

        self.sessions.lock().unwrap().insert(session_id, session);

        // Spawn reader thread for output streaming
        let (tx, rx) = mpsc::unbounded_channel();
        let sessions = self.sessions.clone();

        thread::spawn(move || {
            let mut reader = master.lock().unwrap().try_clone_reader().unwrap();
            let mut buf = [0u8; 8192];

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF - process exited
                        let exit_code = child.lock().unwrap().wait().ok().map(|s| s.exit_code());
                        let _ = tx.send(TerminalEvent::Exit { session_id, exit_code });
                        sessions.lock().unwrap().remove(&session_id);
                        break;
                    }
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        if tx.send(TerminalEvent::Output { session_id, data }).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((session_id, rx))
    }

    /// Send input to a terminal session.
    pub fn send_input(&self, session_id: Uuid, data: &str) -> Result<(), Error> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(&session_id).ok_or(Error::NotFound(session_id))?;

        let mut writer = session.master.lock().unwrap().take_writer().map_err(|e| Error::Pty(e.into()))?;
        writer.write_all(data.as_bytes())?;
        Ok(())
    }

    /// Resize a terminal session.
    pub fn resize(&self, session_id: Uuid, rows: u16, cols: u16) -> Result<(), Error> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(&session_id).ok_or(Error::NotFound(session_id))?;

        session.master.lock().unwrap().resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| Error::Pty(e.into()))?;

        Ok(())
    }

    /// Close a terminal session.
    pub fn close(&self, session_id: Uuid) -> Result<(), Error> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.remove(&session_id).ok_or(Error::NotFound(session_id))?;

        // Kill the child process
        let _ = session.child.lock().unwrap().kill();
        Ok(())
    }

    /// Check if a session exists.
    pub fn exists(&self, session_id: Uuid) -> bool {
        self.sessions.lock().unwrap().contains_key(&session_id)
    }
}

impl Default for TerminalService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn creates_terminal_session() {
        let service = TerminalService::new();
        let (session_id, _rx) = service.create_terminal("/tmp").unwrap();
        assert!(service.exists(session_id));
    }

    #[test]
    fn closes_terminal_session() {
        let service = TerminalService::new();
        let (session_id, _rx) = service.create_terminal("/tmp").unwrap();
        service.close(session_id).unwrap();
        assert!(!service.exists(session_id));
    }

    #[tokio::test]
    async fn sends_input_and_receives_output() {
        let service = TerminalService::new();
        let (session_id, mut rx) = service.create_terminal("/tmp").unwrap();

        // Send echo command
        service.send_input(session_id, "echo hello\n").unwrap();

        // Wait for output
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut got_output = false;
        while let Ok(event) = rx.try_recv() {
            if let TerminalEvent::Output { data, .. } = event {
                if data.contains("hello") {
                    got_output = true;
                    break;
                }
            }
        }

        assert!(got_output, "Should receive echo output");
        service.close(session_id).unwrap();
    }
}
