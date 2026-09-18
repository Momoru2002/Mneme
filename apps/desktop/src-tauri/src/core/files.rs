//! Transport-agnostic core for file operations.
//!
//! Each `pub fn` here takes a plain `&Connection` (no Tauri state) and
//! delegates to `services::files`, mapping errors to sanitized client strings
//! via `client_message()`.  Both the Tauri command wrappers and the web-mode
//! HTTP handlers call into this module, so the security guards apply identically
//! to BOTH transports:
//!  - path-confinement (`path_safety::resolve_well_path`) already lives inside
//!    `services::files`;
//!  - the well-existence guard (`ensure_writable`) is applied here, at the top
//!    of every WRITE op, so a write to an unknown well is rejected over HTTP
//!    exactly as over IPC.

use rusqlite::Connection;

use crate::dto::{FileContent, FileMoveResponse, FileRenameResponse, SaveFileResponse};
use crate::services::files;

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn files_read(conn: &Connection, well_id: &str, path: &str) -> Result<FileContent, String> {
    files::file_read(conn, well_id, path).map_err(|e| e.client_message())
}

pub fn files_create(
    conn: &Connection,
    well_id: &str,
    path: &str,
    content: &str,
) -> Result<SaveFileResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_create(conn, well_id, path, content).map_err(|e| e.client_message())
}

pub fn files_update(
    conn: &Connection,
    well_id: &str,
    path: &str,
    content: &str,
    expected_hash: Option<&str>,
) -> Result<SaveFileResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_update(conn, well_id, path, content, expected_hash).map_err(|e| e.client_message())
}

pub fn files_remove(conn: &Connection, well_id: &str, path: &str) -> Result<(), String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_remove(conn, well_id, path).map_err(|e| e.client_message())
}

pub fn files_rename(
    conn: &Connection,
    well_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<FileRenameResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_rename(conn, well_id, old_path, new_path).map_err(|e| e.client_message())
}

pub fn files_move(
    conn: &Connection,
    well_id: &str,
    source_path: &str,
    dest_path: &str,
) -> Result<FileMoveResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_move(conn, well_id, source_path, dest_path).map_err(|e| e.client_message())
}

pub fn files_duplicate(
    conn: &Connection,
    well_id: &str,
    path: &str,
) -> Result<SaveFileResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    files::file_duplicate(conn, well_id, path).map_err(|e| e.client_message())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::AddWellInput;
    use crate::services::{owner, wells};

    // Returns the TempDir in the tuple so it stays alive for the test scope and
    // is cleaned up on drop (matches the services::files test convention — do NOT
    // use TempDir::keep(), which opts out of cleanup and leaks /tmp dirs).
    fn db_with_well() -> (rusqlite::Connection, String, tempfile::TempDir) {
        let conn = open_in_memory().unwrap();
        let user_id = owner::ensure_owner(&conn).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let well = wells::add(
            &conn,
            &AddWellInput {
                name: "W".into(),
                path: dir.path().to_string_lossy().into_owned(),
                color_tag: None,
            },
            &user_id,
        )
        .unwrap();
        (conn, well.id, dir)
    }

    #[test]
    fn files_create_rejects_a_path_escape() {
        let (conn, well_id, _dir) = db_with_well();
        let err = files_create(&conn, &well_id, "../escape.md", "x").unwrap_err();
        assert_eq!(
            err, "invalid path",
            "a traversal must be rejected, got: {err}"
        );
    }

    #[test]
    fn files_create_then_read_roundtrips_inside_the_well() {
        let (conn, well_id, _dir) = db_with_well();
        files_create(&conn, &well_id, "note.md", "hello").unwrap();
        let got = files_read(&conn, &well_id, "note.md").unwrap();
        assert_eq!(got.content, "hello");
    }
}
