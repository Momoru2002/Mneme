//! Transport-agnostic core for well operations (list / get / activate).
//!
//! Each `pub fn` here takes a plain `&Connection` (no Tauri state) and
//! delegates directly to `services::wells` + `services::owner`, mapping errors
//! to sanitized client strings.  Both the Tauri command wrappers and the HTTP
//! handlers call into this module.
//!
//! NOTE: wells_add / wells_update / wells_remove / wells_validate /
//! wells_browse_host / wells_host_home are NOT in scope for HTTP dispatch and
//! are intentionally omitted here.

use rusqlite::Connection;

use crate::dto::{Well, WellListResponse};
use crate::services::{owner, wells};

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn wells_list(conn: &Connection) -> Result<WellListResponse, String> {
    let user_id = owner::ensure_owner(conn).map_err(|_| "internal error".to_string())?;
    wells::list(conn, &user_id).map_err(|_| "internal error".to_string())
}

pub fn wells_get(conn: &Connection, id: &str) -> Result<Well, String> {
    wells::get(conn, id)
        .map_err(|_| "internal error".to_string())?
        .ok_or_else(|| "well not found".to_string())
}

pub fn wells_activate(conn: &Connection, id: &str) -> Result<(), String> {
    let user_id = owner::ensure_owner(conn).map_err(|_| "internal error".to_string())?;
    wells::activate(conn, &user_id, id).map_err(|e| e.client_message())?;
    Ok(())
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
    fn wells_list_returns_the_registered_well() {
        let (conn, well_id, _dir) = db_with_well();
        let resp = wells_list(&conn).unwrap();
        assert_eq!(resp.wells.len(), 1);
        assert_eq!(resp.wells[0].id, well_id);
        assert!(resp.active_well_id.is_none());
    }

    #[test]
    fn wells_get_returns_well_and_unknown_is_not_found() {
        let (conn, well_id, _dir) = db_with_well();
        let w = wells_get(&conn, &well_id).unwrap();
        assert_eq!(w.id, well_id);
        let err = wells_get(&conn, "ghost").unwrap_err();
        assert_eq!(err, "well not found");
    }

    #[test]
    fn wells_activate_sets_active_well() {
        let (conn, well_id, _dir) = db_with_well();
        wells_activate(&conn, &well_id).unwrap();
        let resp = wells_list(&conn).unwrap();
        assert_eq!(resp.active_well_id.as_deref(), Some(well_id.as_str()));
    }

    #[test]
    fn wells_activate_rejects_unknown_id() {
        let (conn, _well_id, _dir) = db_with_well();
        let err = wells_activate(&conn, "ghost").unwrap_err();
        assert_eq!(err, "well not found");
    }
}
