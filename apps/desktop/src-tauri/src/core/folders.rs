//! Transport-agnostic core for folder operations.
//!
//! Each `pub fn` here takes a plain `&Connection` (no Tauri state) and delegates
//! to `services::folders`, mapping errors via `client_message()`.  Both the Tauri
//! command wrappers and the web-mode HTTP handlers call into this module, so the
//! security guards apply to BOTH transports: path-confinement
//! (`path_safety::resolve_well_path`) already lives in `services::folders`, and
//! the read-only-mirror guard (`ensure_writable`) is applied here — every folder
//! op is a write, so a folder op on a `kind=mirror` well is rejected over HTTP
//! exactly as over IPC.

use rusqlite::Connection;

use crate::dto::{
    FolderCreateInput, FolderCreateResponse, FolderMoveInput, FolderMoveResponse,
    FolderRemoveInput, FolderRenameInput, FolderRenameResponse,
};
use crate::services::folders;

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn folders_create(
    conn: &Connection,
    well_id: &str,
    input: &FolderCreateInput,
) -> Result<FolderCreateResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    folders::folder_create(conn, well_id, input).map_err(|e| e.client_message())
}

pub fn folders_remove(
    conn: &Connection,
    well_id: &str,
    input: &FolderRemoveInput,
) -> Result<(), String> {
    crate::commands::ensure_writable(conn, well_id)?;
    folders::folder_remove(conn, well_id, input).map_err(|e| e.client_message())
}

pub fn folders_rename(
    conn: &Connection,
    well_id: &str,
    input: &FolderRenameInput,
) -> Result<FolderRenameResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    folders::folder_rename(conn, well_id, input).map_err(|e| e.client_message())
}

pub fn folders_move(
    conn: &Connection,
    well_id: &str,
    input: &FolderMoveInput,
) -> Result<FolderMoveResponse, String> {
    crate::commands::ensure_writable(conn, well_id)?;
    folders::folder_move(conn, well_id, input).map_err(|e| e.client_message())
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
    fn folders_create_rejects_a_path_escape() {
        let (conn, well_id, _dir) = db_with_well();
        let err = folders_create(
            &conn,
            &well_id,
            &FolderCreateInput {
                path: "../escape".into(),
            },
        )
        .unwrap_err();
        assert_eq!(
            err, "invalid path",
            "traversal must be rejected, got: {err}"
        );
    }

    #[test]
    fn folders_create_then_verify_directory_exists() {
        let (conn, well_id, dir) = db_with_well();
        let resp = folders_create(
            &conn,
            &well_id,
            &FolderCreateInput {
                path: "notes".into(),
            },
        )
        .unwrap();
        assert_eq!(resp.path, "notes");
        assert!(dir.path().join("notes").is_dir());
    }
}
