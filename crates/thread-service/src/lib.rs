//! Thread service: create, list, rename, and archive conversation threads.

use chrono::Utc;
use models::{Thread, Uuid};
use storage::ThreadStore;

/// Errors from thread operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("thread not found: {0}")]
    NotFound(Uuid),
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

/// Create a new thread in the given project with an optional title.
/// If no title is provided, defaults to "New thread".
pub fn create_thread(
    project_id: Uuid,
    title: Option<String>,
    store: &ThreadStore,
) -> Result<Thread, Error> {
    let now = Utc::now();
    let thread = Thread {
        id: Uuid::new_v4(),
        project_id,
        title: title.unwrap_or_else(|| "New thread".to_string()),
        archived: false,
        created_at: now,
        updated_at: now,
    };

    store.insert(&thread)?;
    Ok(thread)
}

/// List all threads for a project, optionally including archived ones.
pub fn list_threads(
    project_id: Uuid,
    include_archived: bool,
    store: &ThreadStore,
) -> Result<Vec<Thread>, Error> {
    store.list(project_id, include_archived).map_err(Error::from)
}

/// Rename a thread.
pub fn rename_thread(
    thread_id: Uuid,
    new_title: String,
    store: &ThreadStore,
) -> Result<Thread, Error> {
    let mut thread = store
        .get(thread_id)?
        .ok_or(Error::NotFound(thread_id))?;

    thread.title = new_title;
    thread.updated_at = Utc::now();
    store.update(&thread)?;
    Ok(thread)
}

/// Archive a thread (soft delete).
pub fn archive_thread(thread_id: Uuid, store: &ThreadStore) -> Result<Thread, Error> {
    let mut thread = store
        .get(thread_id)?
        .ok_or(Error::NotFound(thread_id))?;

    thread.archived = true;
    thread.updated_at = Utc::now();
    store.update(&thread)?;
    Ok(thread)
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::Database;
    use models::Project;

    fn setup() -> (Database, Uuid) {
        let db = Database::in_memory().unwrap();

        // Create a project first
        let project = Project {
            id: Uuid::new_v4(),
            name: "test".into(),
            path: "/tmp/test".into(),
            is_git_repo: false,
            current_branch: None,
            created_at: Utc::now(),
            last_opened_at: Utc::now(),
        };
        db.projects().upsert(&project).unwrap();

        (db, project.id)
    }

    #[test]
    fn creates_thread_with_default_title() {
        let (db, project_id) = setup();
        let store = db.threads();

        let thread = create_thread(project_id, None, &store).unwrap();
        assert_eq!(thread.title, "New thread");
        assert_eq!(thread.project_id, project_id);
        assert!(!thread.archived);
    }

    #[test]
    fn creates_thread_with_custom_title() {
        let (db, project_id) = setup();
        let store = db.threads();

        let thread = create_thread(project_id, Some("My task".into()), &store).unwrap();
        assert_eq!(thread.title, "My task");
    }

    #[test]
    fn lists_threads_for_project() {
        let (db, project_id) = setup();
        let store = db.threads();

        create_thread(project_id, Some("Thread 1".into()), &store).unwrap();
        create_thread(project_id, Some("Thread 2".into()), &store).unwrap();

        let threads = list_threads(project_id, false, &store).unwrap();
        assert_eq!(threads.len(), 2);
    }

    #[test]
    fn renames_thread() {
        let (db, project_id) = setup();
        let store = db.threads();

        let thread = create_thread(project_id, Some("Old name".into()), &store).unwrap();
        let renamed = rename_thread(thread.id, "New name".into(), &store).unwrap();

        assert_eq!(renamed.title, "New name");
        assert!(renamed.updated_at > thread.updated_at);
    }

    #[test]
    fn archives_thread() {
        let (db, project_id) = setup();
        let store = db.threads();

        let thread = create_thread(project_id, None, &store).unwrap();
        let archived = archive_thread(thread.id, &store).unwrap();

        assert!(archived.archived);

        // Verify it doesn't appear in non-archived list
        let active = list_threads(project_id, false, &store).unwrap();
        assert_eq!(active.len(), 0);

        let all = list_threads(project_id, true, &store).unwrap();
        assert_eq!(all.len(), 1);
    }
}
