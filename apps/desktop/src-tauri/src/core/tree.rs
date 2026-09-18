//! Transport-agnostic core for tree listing.
//!
//! Each `pub fn` here takes a plain `&Connection` (no Tauri state) and
//! delegates directly to `services::tree`, mapping errors to sanitized client
//! strings via `client_message()`.  Both the Tauri command wrappers and the
//! HTTP handlers call into this module — the path-confinement guard
//! (`path_safety::resolve_well_path`) already lives inside `services::tree`,
//! so confinement is enforced for every transport automatically.

use rusqlite::Connection;

use crate::dto::TreeResponse;
use crate::services::tree;

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn tree_list(conn: &Connection, well_id: &str, path: &str) -> Result<TreeResponse, String> {
    tree::tree_list(conn, well_id, path).map_err(|e| e.client_message())
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
    fn tree_list_rejects_path_escape() {
        let (conn, well_id, _dir) = db_with_well();
        let err = tree_list(&conn, &well_id, "../..").unwrap_err();
        assert_eq!(
            err, "invalid path",
            "traversal must be rejected, got: {err}"
        );
    }

    #[test]
    fn tree_list_returns_entries_for_well_root() {
        let (conn, well_id, dir) = db_with_well();
        std::fs::write(dir.path().join("note.md"), b"# Hello").unwrap();
        let resp = tree_list(&conn, &well_id, "").unwrap();
        assert_eq!(resp.well_id, well_id);
        assert_eq!(resp.entries.len(), 1);
        assert_eq!(resp.entries[0].name, "note.md");
    }
}
