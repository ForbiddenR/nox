//! Terminal manager: coordinates PTY service with storage and event routing.

use models::{TerminalSession, Uuid};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use terminal_service::{TerminalEvent, TerminalService};
use tokio::sync::mpsc;

/// Notification callback for terminal events.
pub type NotificationSender = Arc<dyn Fn(String, serde_json::Value) + Send + Sync>;

/// Manages terminal sessions with event routing.
pub struct TerminalManager {
    service: TerminalService,
    sessions: Arc<Mutex<HashMap<Uuid, mpsc::UnboundedReceiver<TerminalEvent>>>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            service: TerminalService::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a terminal session and start routing events.
    pub fn create_terminal(
        &self,
        session: TerminalSession,
        notify: NotificationSender,
    ) -> Result<Uuid, terminal_service::Error> {
        let (session_id, rx) = self.service.create_terminal(&session.cwd)?;

        // Store the receiver
        self.sessions.lock().unwrap().insert(session_id, rx);

        // Spawn task to forward events
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut rx = {
                let mut map = sessions.lock().unwrap();
                map.remove(&session_id).unwrap()
            };

            while let Some(event) = rx.recv().await {
                match &event {
                    TerminalEvent::Output { session_id, data } => {
                        notify(
                            "terminal:output".to_string(),
                            serde_json::json!({
                                "session_id": session_id.to_string(),
                                "data": data,
                            }),
                        );
                    }
                    TerminalEvent::Exit { session_id, exit_code } => {
                        notify(
                            "terminal:exit".to_string(),
                            serde_json::json!({
                                "session_id": session_id.to_string(),
                                "exit_code": exit_code,
                            }),
                        );
                        break;
                    }
                }
            }
        });

        Ok(session_id)
    }

    /// Send input to a terminal.
    pub fn send_input(&self, session_id: Uuid, data: &str) -> Result<(), terminal_service::Error> {
        self.service.send_input(session_id, data)
    }

    /// Resize a terminal.
    pub fn resize(&self, session_id: Uuid, rows: u16, cols: u16) -> Result<(), terminal_service::Error> {
        self.service.resize(session_id, rows, cols)
    }

    /// Close a terminal.
    pub fn close(&self, session_id: Uuid) -> Result<(), terminal_service::Error> {
        self.service.close(session_id)
    }

    /// Check if a session exists.
    pub fn exists(&self, session_id: Uuid) -> bool {
        self.service.exists(session_id)
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
