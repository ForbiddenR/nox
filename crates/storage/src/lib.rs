//! SQLite-backed persistence layer for projects, threads, runs, and artifacts.
//!
//! The database is automatically created in the app data directory and migrated
//! on first open. All writes are synchronous; the sidecar is single-threaded and
//! blocking I/O is fine for local SQLite.

mod migrations;
mod projects;
mod threads;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub use projects::ProjectStore;
pub use threads::ThreadStore;

/// The central database handle. Owns the SQLite connection and exposes
/// type-safe stores for each domain.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path, applying all migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for tests).
    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn projects(&self) -> ProjectStore<'_> {
        ProjectStore::new(&self.conn)
    }

    pub fn threads(&self) -> ThreadStore<'_> {
        ThreadStore::new(&self.conn)
    }
}

/// Returns the default database path for the application.
/// On macOS: `~/Library/Application Support/cox/cox.db`
/// On Linux: `~/.local/share/cox/cox.db`
/// On Windows: `{FOLDERID_LocalAppData}\cox\cox.db`
pub fn default_db_path() -> Result<PathBuf, std::io::Error> {
    let data_dir = if cfg!(target_os = "macos") {
        dirs::home_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home dir"))?
            .join("Library/Application Support/cox")
    } else if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no local data dir"))?
            .join("cox")
    } else {
        dirs::home_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home dir"))?
            .join(".local/share/cox")
    };

    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("cox.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_db() {
        let db = Database::in_memory().unwrap();
        // Just verify the connection is valid.
        db.conn.execute_batch("SELECT 1;").unwrap();
    }
}
