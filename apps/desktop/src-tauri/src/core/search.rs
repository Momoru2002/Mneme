//! Transport-agnostic core for search.
//!
//! `search_query` takes a plain `&Connection` (no Tauri state) and delegates to
//! `services::search`, mapping errors to sanitized client strings.  Both the
//! Tauri command wrapper and the web-mode HTTP handler call into this module —
//! path confinement (`path_safety::resolve_well_path`) already lives inside the
//! service, and the query-length cap is enforced here so it applies to BOTH
//! transports.  Search is read-only, so there is no read-only-mirror guard.

use rusqlite::Connection;

use crate::dto::{SearchOptions, SearchResponse};
use crate::services::search;

/// Maximum query length — mirrors `z.string().max(200)` in the slice-6 route.
const MAX_Q_LEN: usize = 200;

pub fn search_query(
    conn: &Connection,
    well_id: &str,
    q: &str,
    include_content: bool,
    options: &SearchOptions,
) -> Result<SearchResponse, String> {
    if q.len() > MAX_Q_LEN {
        return Err(format!("q must be at most {MAX_Q_LEN} characters"));
    }
    search::search_well(conn, well_id, q, include_content, options).map_err(|e| e.client_message())
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
    fn search_query_finds_a_filename_match() {
        let (conn, well_id, dir) = db_with_well();
        std::fs::write(dir.path().join("hello.md"), b"# Hi").unwrap();
        let resp = search_query(&conn, &well_id, "hello", true, &SearchOptions::default()).unwrap();
        assert!(
            resp.results.iter().any(|r| r.path == "hello.md"),
            "expected a match for hello.md, got: {:?}",
            resp.results
        );
    }

    #[test]
    fn search_query_rejects_an_overlong_query() {
        let (conn, well_id, _dir) = db_with_well();
        let long = "x".repeat(MAX_Q_LEN + 1);
        let err =
            search_query(&conn, &well_id, &long, true, &SearchOptions::default()).unwrap_err();
        assert!(err.contains("at most"), "got: {err}");
    }
}
