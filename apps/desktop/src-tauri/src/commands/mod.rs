//! Tauri command surface — thin `#[tauri::command]` wrappers over `crate::services`.

pub mod auth;
pub mod files;
pub mod folders;
pub mod mcp;
pub mod search;
pub mod settings;
pub mod templates;
pub mod tree;
pub mod web_mode;
pub mod wells;

/// Acquire the single application database connection.
///
/// rusqlite is synchronous and Tauri commands are `async`; all command handlers
/// lock the connection for the duration of their DB work. Contention is
/// negligible for a single-user desktop app. The lock is released when the
/// returned guard drops (end of the owning command scope).
pub(crate) fn lock_db<'a>(
    db: &'a tauri::State<'a, crate::db::Db>,
) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, String> {
    db.0.lock()
        .map_err(|_| "database lock poisoned".to_string())
}

/// Guard: reject a write to a well that does not exist.
///
/// Called at the top of every mutating file/folder command, after acquiring the
/// DB connection, before any filesystem side-effect. All wells are now plainly
/// editable, so this is purely an existence check shared by both the Tauri IPC
/// and the web-mode HTTP transports.
pub(crate) fn ensure_writable(conn: &rusqlite::Connection, well_id: &str) -> Result<(), String> {
    crate::services::wells::get(conn, well_id)
        .map_err(|_| "internal error".to_string())?
        .ok_or_else(|| "well not found".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::AddWellInput;
    use crate::services::{owner, wells};

    /// A migrated in-memory DB with one owner + one normal well.
    fn db_with_well() -> (rusqlite::Connection, String) {
        let conn = open_in_memory().unwrap();
        let user_id = owner::ensure_owner(&conn).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.keep();
        let well = wells::add(
            &conn,
            &AddWellInput {
                name: "TestWell".into(),
                path: path.to_string_lossy().into_owned(),
                color_tag: None,
            },
            &user_id,
        )
        .unwrap();
        (conn, well.id)
    }

    #[test]
    fn ensure_writable_allows_normal_well() {
        let (conn, well_id) = db_with_well();
        assert!(
            ensure_writable(&conn, &well_id).is_ok(),
            "a normal well must be writable"
        );
    }

    #[test]
    fn ensure_writable_rejects_unknown_well() {
        let (conn, _) = db_with_well();
        let err = ensure_writable(&conn, "nonexistent-id").unwrap_err();
        assert!(
            err.contains("not found"),
            "error message must mention not found, got: {err}"
        );
    }
}
