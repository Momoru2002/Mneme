//! Wells = tracked directories of knowledge. DB registry + host-filesystem
//! validation/browsing. Ports the slice-6 vault module (now Well).
//!
//! Path-safety note: this module REGISTERS a host directory as a Well and lets
//! the user browse the host fs to pick one (a folder picker — broad by design).
//! It guards null-bytes + requires absolute paths, and only registers a path
//! that is a real readable directory. The stricter *intra-well* confinement
//! (resolving file paths so they cannot escape the Well root) belongs to the
//! Files/Tree modules. The whole surface gets a dedicated path-safety review.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::clock::now_ms;
use crate::db::error::DbError;
use crate::dto::{
    AddWellInput, HostBrowseEntry, HostBrowseResponse, UpdateWellInput, Well, WellListResponse,
    WellValidationResult,
};

#[derive(Debug, thiserror::Error)]
pub enum WellsError {
    #[error("well not found")]
    NotFound,
    #[error("a well with that path already exists")]
    AlreadyExists,
    #[error("invalid: {0}")]
    Invalid(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl WellsError {
    pub fn client_message(&self) -> String {
        match self {
            WellsError::NotFound => "well not found".into(),
            WellsError::AlreadyExists => "a well with that path already exists".into(),
            WellsError::Invalid(m) => format!("invalid: {m}"),
            WellsError::Db(_) => "internal error".into(),
        }
    }
}

#[derive(Debug, Clone)]
struct WellRow {
    id: String,
    name: String,
    path: String,
    color_tag: Option<String>,
    sort_order: i64,
    last_accessed_at: i64,
    created_at: i64,
    updated_at: i64,
}

const COLS: &str = "id, name, path, color_tag, sort_order, last_accessed_at, created_at, updated_at";

fn map_well(r: &rusqlite::Row<'_>) -> rusqlite::Result<WellRow> {
    Ok(WellRow {
        id: r.get(0)?,
        name: r.get(1)?,
        path: r.get(2)?,
        color_tag: r.get(3)?,
        sort_order: r.get(4)?,
        last_accessed_at: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

fn to_well(w: &WellRow) -> Well {
    Well {
        id: w.id.clone(),
        name: w.name.clone(),
        path: w.path.clone(),
        color_tag: w.color_tag.clone(),
        sort_order: w.sort_order,
        last_accessed_at: w.last_accessed_at,
        created_at: w.created_at,
        updated_at: w.updated_at,
    }
}

// --- input validation (mirrors shared/schemas/well.ts) ---

fn validate_name(name: &str) -> Result<(), WellsError> {
    let n = name.chars().count();
    if (1..=64).contains(&n) {
        Ok(())
    } else {
        Err(WellsError::Invalid("name must be 1..=64 characters".into()))
    }
}

fn validate_path_input(path: &str) -> Result<(), WellsError> {
    if path.is_empty() {
        return Err(WellsError::Invalid("path is required".into()));
    }
    if path.contains('\0') {
        return Err(WellsError::Invalid("path contains a null byte".into()));
    }
    if !Path::new(path).is_absolute() {
        return Err(WellsError::Invalid("path must be absolute".into()));
    }
    Ok(())
}

fn validate_color(color: &Option<String>) -> Result<(), WellsError> {
    if let Some(c) = color {
        let ok =
            c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit());
        if !ok {
            return Err(WellsError::Invalid(
                "colorTag must be a hex like #4FD1C5".into(),
            ));
        }
    }
    Ok(())
}

// --- DB registry ---

pub fn list(conn: &Connection, user_id: &str) -> Result<WellListResponse, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM wells ORDER BY sort_order ASC, last_accessed_at DESC"
    ))?;
    let wells = stmt
        .query_map([], map_well)?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .map(to_well)
        .collect();
    Ok(WellListResponse {
        wells,
        active_well_id: get_active_well_id(conn, user_id)?,
    })
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Well>, DbError> {
    Ok(find_by_id(conn, id)?.as_ref().map(to_well))
}

fn find_by_id(conn: &Connection, id: &str) -> Result<Option<WellRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM wells WHERE id = ?1"),
            [id],
            map_well,
        )
        .optional()?)
}

fn find_by_path(conn: &Connection, path: &str) -> Result<Option<WellRow>, DbError> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLS} FROM wells WHERE path = ?1"),
            [path],
            map_well,
        )
        .optional()?)
}

pub fn add(
    conn: &Connection,
    input: &AddWellInput,
    created_by_user_id: &str,
) -> Result<Well, WellsError> {
    validate_name(&input.name)?;
    validate_path_input(&input.path)?;
    validate_color(&input.color_tag)?;

    // The registered path must be a real, readable directory — the Well root is
    // the trust anchor for all later file operations.
    let v = validate_well_path(&input.path);
    if !(v.exists && v.is_directory && v.readable) {
        return Err(WellsError::Invalid(
            v.error
                .unwrap_or_else(|| "path is not a readable directory".into()),
        ));
    }
    // Canonicalize: store the normalized, symlink-resolved absolute path so the
    // Well-root is a sound trust anchor for later intra-well file ops, and so
    // `/a/../a`, a trailing `/.`, and symlink aliases all dedup to one Well
    // (D-WELL-REV: non-canonical paths previously slipped past the dup check).
    let canonical = std::fs::canonicalize(&input.path)
        .map_err(|e| WellsError::Invalid(format!("cannot resolve path: {e}")))?
        .to_string_lossy()
        .into_owned();
    if find_by_path(conn, &canonical)?.is_some() {
        return Err(WellsError::AlreadyExists);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_ms();
    conn.execute(
        "INSERT INTO wells (id, name, path, color_tag, sort_order, last_accessed_at, created_by_user_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?5, ?5)",
        rusqlite::params![id, input.name, canonical, input.color_tag, now, created_by_user_id],
    )
    .map_err(DbError::from)?;
    let row = find_by_id(conn, &id)?.ok_or(WellsError::NotFound)?;
    Ok(to_well(&row))
}

pub fn update(conn: &Connection, id: &str, input: &UpdateWellInput) -> Result<Well, WellsError> {
    if let Some(name) = &input.name {
        validate_name(name)?;
    }
    validate_color(&input.color_tag)?;
    // COALESCE: a None field is left unchanged (color cannot be cleared to NULL).
    let n = conn
        .execute(
            "UPDATE wells SET name = COALESCE(?2, name), color_tag = COALESCE(?3, color_tag), updated_at = ?4 WHERE id = ?1",
            rusqlite::params![id, input.name, input.color_tag, now_ms()],
        )
        .map_err(DbError::from)?;
    if n == 0 {
        return Err(WellsError::NotFound);
    }
    let row = find_by_id(conn, id)?.ok_or(WellsError::NotFound)?;
    Ok(to_well(&row))
}

pub fn remove(conn: &Connection, id: &str) -> Result<(), WellsError> {
    let n = conn
        .execute("DELETE FROM wells WHERE id = ?1", [id])
        .map_err(DbError::from)?;
    if n == 0 {
        return Err(WellsError::NotFound);
    }
    Ok(())
}

pub fn get_active_well_id(conn: &Connection, user_id: &str) -> Result<Option<String>, DbError> {
    Ok(conn
        .query_row(
            "SELECT well_id FROM active_well WHERE user_id = ?1",
            [user_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

pub fn activate(conn: &Connection, user_id: &str, well_id: &str) -> Result<(), WellsError> {
    if find_by_id(conn, well_id)?.is_none() {
        return Err(WellsError::NotFound);
    }
    let now = now_ms();
    conn.execute(
        "INSERT INTO active_well (user_id, well_id, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(user_id) DO UPDATE SET well_id = excluded.well_id, updated_at = excluded.updated_at",
        rusqlite::params![user_id, well_id, now],
    )
    .map_err(DbError::from)?;
    conn.execute(
        "UPDATE wells SET last_accessed_at = ?2 WHERE id = ?1",
        rusqlite::params![well_id, now],
    )
    .map_err(DbError::from)?;
    Ok(())
}

// --- host filesystem ---

/// Inspect a candidate Well directory (existence / dir / readable / file count /
/// .obsidian marker). Never throws; surfaces problems in `error`.
pub fn validate_well_path(path: &str) -> WellValidationResult {
    let mut r = WellValidationResult::default();
    if path.contains('\0') {
        r.error = Some("path contains a null byte".into());
        return r;
    }
    // is_absolute guard lives HERE (not only in callers) so the pub fn is safe
    // regardless of call site — e.g. wells_validate calls it directly
    // (D-WELL-REV: a relative path would otherwise probe the process cwd).
    if !Path::new(path).is_absolute() {
        r.error = Some("path must be absolute".into());
        return r;
    }
    let p = Path::new(path);
    let meta = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return r, // exists stays false
    };
    r.exists = true;
    r.is_directory = meta.is_dir();
    if !r.is_directory {
        r.error = Some("path is not a directory".into());
        return r;
    }
    match std::fs::read_dir(p) {
        Ok(entries) => {
            r.readable = true;
            for e in entries.flatten() {
                match e.file_type() {
                    Ok(ft) if ft.is_file() => r.file_count += 1,
                    Ok(ft) if ft.is_dir() && e.file_name() == ".obsidian" => {
                        r.is_obsidian_well = true
                    }
                    _ => {}
                }
            }
        }
        Err(e) => r.error = Some(e.to_string()),
    }
    r
}

fn dir_has_subdir(p: &Path) -> bool {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .any(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        })
        .unwrap_or(false)
}

/// List the subdirectories of an absolute host path (the folder picker).
pub fn browse_host(path: &str) -> Result<HostBrowseResponse, WellsError> {
    validate_path_input(path)?;
    let p = Path::new(path);
    let rd = std::fs::read_dir(p).map_err(|e| WellsError::Invalid(e.to_string()))?;
    let mut entries: Vec<HostBrowseEntry> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| {
            let full = e.path();
            HostBrowseEntry {
                has_children: dir_has_subdir(&full),
                name: e.file_name().to_string_lossy().into_owned(),
                path: full.to_string_lossy().into_owned(),
            }
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(HostBrowseResponse {
        parent: p.parent().map(|pp| pp.to_string_lossy().into_owned()),
        path: path.to_string(),
        entries,
    })
}

/// Folder picker's starting point: the user's home directory.
pub fn host_home() -> Result<HostBrowseResponse, WellsError> {
    let home = dirs::home_dir()
        .ok_or_else(|| WellsError::Invalid("could not determine the user's home directory".into()))?;
    browse_host(&home.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn db_with_user() -> (Connection, String) {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u1', 'alice', 'h', 'Alice', 0, 0)",
            [],
        )
        .unwrap();
        (conn, "u1".to_string())
    }

    fn add_input(dir: &std::path::Path, name: &str) -> AddWellInput {
        AddWellInput {
            name: name.into(),
            path: dir.to_string_lossy().into_owned(),
            color_tag: Some("#4FD1C5".into()),
        }
    }

    #[test]
    fn add_validates_real_dir_then_lists() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        let w = add(&conn, &add_input(dir.path(), "Research"), &uid).unwrap();
        assert_eq!(w.name, "Research");
        // stored path is the canonical (symlink-resolved) form
        assert_eq!(
            w.path,
            std::fs::canonicalize(dir.path()).unwrap().to_string_lossy()
        );
        assert_eq!(w.color_tag.as_deref(), Some("#4FD1C5"));

        let resp = list(&conn, &uid).unwrap();
        assert_eq!(resp.wells.len(), 1);
        assert!(resp.active_well_id.is_none());
    }

    #[test]
    fn add_rejects_nonexistent_or_file_path() {
        let (conn, uid) = db_with_user();
        let r = add(
            &conn,
            &AddWellInput {
                name: "X".into(),
                path: "/no/such/dir/xyz".into(),
                color_tag: None,
            },
            &uid,
        );
        assert!(matches!(r, Err(WellsError::Invalid(_))));

        // a file, not a directory
        let f = tempfile::NamedTempFile::new().unwrap();
        let r2 = add(
            &conn,
            &AddWellInput {
                name: "X".into(),
                path: f.path().to_string_lossy().into_owned(),
                color_tag: None,
            },
            &uid,
        );
        assert!(matches!(r2, Err(WellsError::Invalid(_))));
    }

    #[test]
    fn add_rejects_relative_path_and_bad_color() {
        let (conn, uid) = db_with_user();
        assert!(matches!(
            add(
                &conn,
                &AddWellInput {
                    name: "X".into(),
                    path: "relative/dir".into(),
                    color_tag: None
                },
                &uid
            ),
            Err(WellsError::Invalid(_))
        ));
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            add(
                &conn,
                &AddWellInput {
                    name: "X".into(),
                    path: dir.path().to_string_lossy().into_owned(),
                    color_tag: Some("red".into())
                },
                &uid
            ),
            Err(WellsError::Invalid(_))
        ));
    }

    #[test]
    fn add_rejects_duplicate_path() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        assert!(matches!(
            add(&conn, &add_input(dir.path(), "B"), &uid),
            Err(WellsError::AlreadyExists)
        ));
    }

    #[test]
    fn update_changes_fields_and_not_found() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        let w = add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        let u = update(
            &conn,
            &w.id,
            &UpdateWellInput {
                name: Some("Renamed".into()),
                color_tag: Some("#000000".into()),
            },
        )
        .unwrap();
        assert_eq!(u.name, "Renamed");
        assert_eq!(u.color_tag.as_deref(), Some("#000000"));
        assert!(matches!(
            update(&conn, "ghost", &UpdateWellInput::default()),
            Err(WellsError::NotFound)
        ));
    }

    #[test]
    fn remove_and_not_found() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        let w = add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        remove(&conn, &w.id).unwrap();
        assert!(get(&conn, &w.id).unwrap().is_none());
        assert!(matches!(remove(&conn, "ghost"), Err(WellsError::NotFound)));
    }

    #[test]
    fn activate_sets_active_and_rejects_unknown() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        let w = add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        activate(&conn, &uid, &w.id).unwrap();
        assert_eq!(
            list(&conn, &uid).unwrap().active_well_id.as_deref(),
            Some(w.id.as_str())
        );
        assert!(matches!(
            activate(&conn, &uid, "ghost"),
            Err(WellsError::NotFound)
        ));
    }

    #[test]
    fn validate_well_path_reports_dir_state() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".obsidian")).unwrap();
        std::fs::write(dir.path().join("note.md"), b"x").unwrap();
        let v = validate_well_path(&dir.path().to_string_lossy());
        assert!(v.exists && v.is_directory && v.readable);
        assert!(v.is_obsidian_well);
        assert_eq!(v.file_count, 1);

        let missing = validate_well_path("/no/such/path/zzz");
        assert!(!missing.exists);

        assert!(validate_well_path("/etc/hosts").error.is_some()); // not a directory
    }

    #[test]
    fn browse_host_lists_subdirs_with_parent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("alpha")).unwrap();
        std::fs::create_dir(dir.path().join("alpha").join("child")).unwrap();
        std::fs::create_dir(dir.path().join("beta")).unwrap();
        std::fs::write(dir.path().join("file.txt"), b"x").unwrap();

        let resp = browse_host(&dir.path().to_string_lossy()).unwrap();
        let names: Vec<_> = resp.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta"]); // only dirs, sorted, no file
        assert!(
            resp.entries
                .iter()
                .find(|e| e.name == "alpha")
                .unwrap()
                .has_children
        );
        assert!(
            !resp
                .entries
                .iter()
                .find(|e| e.name == "beta")
                .unwrap()
                .has_children
        );
        assert!(resp.parent.is_some());

        assert!(matches!(
            browse_host("relative"),
            Err(WellsError::Invalid(_))
        ));
    }

    // --- path-safety review fixes (D-WELL-REV) ---

    #[test]
    fn add_canonicalizes_and_dedups_alias_paths() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        // "<dir>/." resolves to the same directory → must dedup to AlreadyExists
        let alias = format!("{}/.", dir.path().display());
        assert!(matches!(
            add(
                &conn,
                &AddWellInput {
                    name: "B".into(),
                    path: alias,
                    color_tag: None
                },
                &uid
            ),
            Err(WellsError::AlreadyExists)
        ));
    }

    #[test]
    fn validate_well_path_rejects_relative_path() {
        let v = validate_well_path("relative/dir");
        assert!(!v.exists);
        assert_eq!(v.error.as_deref(), Some("path must be absolute"));
    }

    #[test]
    fn removing_the_active_well_clears_the_active_pointer() {
        let (conn, uid) = db_with_user();
        let dir = tempfile::tempdir().unwrap();
        let w = add(&conn, &add_input(dir.path(), "A"), &uid).unwrap();
        activate(&conn, &uid, &w.id).unwrap();
        remove(&conn, &w.id).unwrap();
        // active_well.well_id is ON DELETE SET NULL → the active pointer clears
        assert!(get_active_well_id(&conn, &uid).unwrap().is_none());
        assert!(list(&conn, &uid).unwrap().active_well_id.is_none());
    }
}
