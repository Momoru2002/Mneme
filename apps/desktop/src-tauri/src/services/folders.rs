//! Folder operations inside a Well: create / remove / rename / move.
//! Every path is confined to the Well root via `path_safety::resolve_well_path`.
//! Ports `apps/api/src/services/folder-service.ts` 1:1 (adapting Node/Postgres
//! idioms to Rust + std::fs).

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::dto::{
    FolderCreateInput, FolderCreateResponse, FolderMoveInput, FolderMoveResponse,
    FolderRemoveInput, FolderRenameInput, FolderRenameResponse,
};
use crate::path_safety::{resolve_well_path, PathSafetyError};
use crate::services::wells;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum FoldersError {
    #[error("well not found")]
    WellNotFound,
    #[error("folder not found")]
    NotFound,
    #[error("folder already exists")]
    AlreadyExists,
    #[error("path is not a directory")]
    NotADirectory,
    /// Returned when a non-recursive remove is attempted on a non-empty
    /// directory.  Ports the `'not_empty'` code from `FolderServiceError` in
    /// `apps/api/src/services/folder-service.ts:9`.
    #[error("folder is not empty")]
    NotEmpty,
    /// Attempted to operate directly on the well root (path "" or ".").
    #[error("cannot operate on the well root")]
    WellRoot,
    #[error("invalid path")]
    PathSafety(#[from] PathSafetyError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

impl FoldersError {
    pub fn client_message(&self) -> String {
        match self {
            FoldersError::WellNotFound => "well not found".into(),
            FoldersError::NotFound => "folder not found".into(),
            FoldersError::AlreadyExists => "folder already exists".into(),
            FoldersError::NotADirectory => "path is not a directory".into(),
            FoldersError::NotEmpty => "folder is not empty".into(),
            FoldersError::WellRoot => "cannot operate on the well root".into(),
            FoldersError::PathSafety(_) => "invalid path".into(),
            FoldersError::Db(_) | FoldersError::Io(_) => "internal error".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fetch the well and resolve `rel_path` against its root, returning the
/// absolute path or an error — the single chokepoint for path confinement.
fn resolve(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
) -> Result<std::path::PathBuf, FoldersError> {
    let well = wells::get(conn, well_id)?.ok_or(FoldersError::WellNotFound)?;
    Ok(resolve_well_path(&well.path, rel_path)?)
}

/// Guard against operating on the well root itself.  `abs` must be the
/// resolved absolute path; `well_root` is the canonical well root path.
/// Returns `Err(FoldersError::WellRoot)` when `abs == well_root`.
fn reject_well_root(abs: &std::path::Path, well_root: &str) -> Result<(), FoldersError> {
    if abs == std::path::Path::new(well_root) {
        return Err(FoldersError::WellRoot);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Operations (port of folder-service.ts)
// ---------------------------------------------------------------------------

/// Create a directory (and any intermediate parents — mirrors `mkdir recursive`).
/// Returns the relative path on success.
pub fn folder_create(
    conn: &Connection,
    well_id: &str,
    input: &FolderCreateInput,
) -> Result<FolderCreateResponse, FoldersError> {
    let abs = resolve(conn, well_id, &input.path)?;
    if abs.exists() {
        return Err(FoldersError::AlreadyExists);
    }
    std::fs::create_dir_all(&abs)?;
    Ok(FolderCreateResponse {
        path: input.path.clone(),
    })
}

/// Remove a directory.  When `confirm` is false, the remove fails if the
/// directory is non-empty (mirrors `DELETE /api/folders?confirm=false`).
///
/// Operating on the well root (`path=""` or `path="."`) is rejected with
/// `WellRoot` to prevent accidental vault erasure.
pub fn folder_remove(
    conn: &Connection,
    well_id: &str,
    input: &FolderRemoveInput,
) -> Result<(), FoldersError> {
    let well = wells::get(conn, well_id)?.ok_or(FoldersError::WellNotFound)?;
    let abs = resolve_well_path(&well.path, &input.path)?;

    // Forbid operating on the well root itself (path="" or path=".").
    reject_well_root(&abs, &well.path)?;

    if !abs.exists() {
        return Err(FoldersError::NotFound);
    }
    if !abs.is_dir() {
        return Err(FoldersError::NotADirectory);
    }
    if input.confirm {
        std::fs::remove_dir_all(&abs)?;
    } else {
        std::fs::remove_dir(&abs).map_err(|e| {
            // `ErrorKind::DirectoryNotEmpty` (stable since Rust 1.83) instead of
            // matching a Unix-only raw OS error code — this crate's pinned
            // toolchain (rust-toolchain.toml) is 1.92, well above that, and this
            // is the only form that also works correctly on Windows.
            if e.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                FoldersError::NotEmpty
            } else {
                FoldersError::Io(e)
            }
        })?;
    }
    Ok(())
}

/// Rename a folder (old_path → new_path, both relative to the well root).
/// Parent directories of new_path are created if they don't exist.
///
/// Operating on the well root (`old_path=""` or `old_path="."`) is rejected
/// with `WellRoot`.  The source must be a directory; passing a file path
/// returns `NotADirectory`.
pub fn folder_rename(
    conn: &Connection,
    well_id: &str,
    input: &FolderRenameInput,
) -> Result<FolderRenameResponse, FoldersError> {
    let well = wells::get(conn, well_id)?.ok_or(FoldersError::WellNotFound)?;
    let abs_old = resolve_well_path(&well.path, &input.old_path)?;
    let abs_new = resolve_well_path(&well.path, &input.new_path)?;

    // Forbid renaming the well root itself.
    reject_well_root(&abs_old, &well.path)?;

    if !abs_old.exists() {
        return Err(FoldersError::NotFound);
    }
    // Guard: only operate on directories, not regular files.
    if !abs_old.is_dir() {
        return Err(FoldersError::NotADirectory);
    }
    if abs_new.exists() {
        return Err(FoldersError::AlreadyExists);
    }
    // Ensure the parent of the destination exists.
    if let Some(parent) = abs_new.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::rename(&abs_old, &abs_new)?;
    Ok(FolderRenameResponse {
        old_path: input.old_path.clone(),
        new_path: input.new_path.clone(),
    })
}

/// Move a folder (source_path → dest_path, both relative to the well root).
/// Internally delegates to folder_rename — same semantics, different response shape.
pub fn folder_move(
    conn: &Connection,
    well_id: &str,
    input: &FolderMoveInput,
) -> Result<FolderMoveResponse, FoldersError> {
    let rename_input = FolderRenameInput {
        old_path: input.source_path.clone(),
        new_path: input.dest_path.clone(),
    };
    folder_rename(conn, well_id, &rename_input)?;
    Ok(FolderMoveResponse {
        source_path: input.source_path.clone(),
        dest_path: input.dest_path.clone(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::AddWellInput;

    /// Provision an in-memory DB with one user + one well, return (conn, well_id, tempdir).
    fn setup() -> (Connection, String, tempfile::TempDir) {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u1', 'alice', 'h', 'Alice', 0, 0)",
            [],
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let well = crate::services::wells::add(
            &conn,
            &AddWellInput {
                name: "W".into(),
                path: dir.path().to_string_lossy().into_owned(),
                color_tag: None,
            },
            "u1",
        )
        .unwrap();
        (conn, well.id, dir)
    }

    // --- folder_create ---

    #[test]
    fn create_makes_the_directory() {
        let (conn, wid, dir) = setup();
        let resp = folder_create(
            &conn,
            &wid,
            &FolderCreateInput {
                path: "notes".into(),
            },
        )
        .unwrap();
        assert_eq!(resp.path, "notes");
        assert!(dir.path().join("notes").is_dir());
    }

    #[test]
    fn create_with_nested_path_uses_create_dir_all() {
        let (conn, wid, dir) = setup();
        folder_create(
            &conn,
            &wid,
            &FolderCreateInput {
                path: "a/b/c".into(),
            },
        )
        .unwrap();
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn create_rejects_already_existing_dir() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("existing")).unwrap();
        let err = folder_create(
            &conn,
            &wid,
            &FolderCreateInput {
                path: "existing".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::AlreadyExists));
    }

    #[test]
    fn create_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        let err = folder_create(
            &conn,
            &wid,
            &FolderCreateInput {
                path: "../escape".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::PathSafety(_)));
    }

    #[test]
    fn create_rejects_unknown_well() {
        let (conn, _wid, _dir) = setup();
        let err =
            folder_create(&conn, "ghost", &FolderCreateInput { path: "x".into() }).unwrap_err();
        assert!(matches!(err, FoldersError::WellNotFound));
    }

    // --- folder_remove ---

    #[test]
    fn remove_deletes_an_empty_dir() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("todelete")).unwrap();
        folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "todelete".into(),
                confirm: false,
            },
        )
        .unwrap();
        assert!(!dir.path().join("todelete").exists());
    }

    #[test]
    fn remove_confirm_deletes_non_empty_dir() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir_all(dir.path().join("tree/sub")).unwrap();
        std::fs::write(dir.path().join("tree/sub/file.md"), b"x").unwrap();
        folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "tree".into(),
                confirm: true,
            },
        )
        .unwrap();
        assert!(!dir.path().join("tree").exists());
    }

    #[test]
    fn remove_non_confirm_fails_on_non_empty_dir_with_not_empty() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir_all(dir.path().join("nonempty/child")).unwrap();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "nonempty".into(),
                confirm: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::NotEmpty));
    }

    #[test]
    fn remove_not_found_returns_error() {
        let (conn, wid, _dir) = setup();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "ghost".into(),
                confirm: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::NotFound));
    }

    #[test]
    fn remove_file_path_returns_not_a_directory() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("file.md"), b"x").unwrap();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "file.md".into(),
                confirm: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::NotADirectory));
    }

    #[test]
    fn remove_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "../escape".into(),
                confirm: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::PathSafety(_)));
    }

    #[test]
    fn remove_dot_path_rejects_well_root() {
        let (conn, wid, _dir) = setup();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: ".".into(),
                confirm: true,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::WellRoot));
    }

    #[test]
    fn remove_empty_path_rejects_well_root() {
        let (conn, wid, _dir) = setup();
        let err = folder_remove(
            &conn,
            &wid,
            &FolderRemoveInput {
                path: "".into(),
                confirm: true,
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::WellRoot));
    }

    // --- folder_rename ---

    #[test]
    fn rename_moves_the_directory() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("old")).unwrap();
        let resp = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "old".into(),
                new_path: "new".into(),
            },
        )
        .unwrap();
        assert_eq!(resp.old_path, "old");
        assert_eq!(resp.new_path, "new");
        assert!(!dir.path().join("old").exists());
        assert!(dir.path().join("new").is_dir());
    }

    #[test]
    fn rename_creates_missing_parent_of_dest() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "src".into(),
                new_path: "a/b/dest".into(),
            },
        )
        .unwrap();
        assert!(dir.path().join("a/b/dest").is_dir());
    }

    #[test]
    fn rename_source_not_found() {
        let (conn, wid, _dir) = setup();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "ghost".into(),
                new_path: "new".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::NotFound));
    }

    #[test]
    fn rename_dest_already_exists() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "a".into(),
                new_path: "b".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::AlreadyExists));
    }

    #[test]
    fn rename_rejects_traversal_on_old_path() {
        let (conn, wid, _dir) = setup();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "../escape".into(),
                new_path: "new".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::PathSafety(_)));
    }

    #[test]
    fn rename_rejects_traversal_on_new_path() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "src".into(),
                new_path: "../escape".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::PathSafety(_)));
    }

    #[test]
    fn rename_file_source_returns_not_a_directory() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("file.md"), b"x").unwrap();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "file.md".into(),
                new_path: "renamed".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::NotADirectory));
    }

    #[test]
    fn rename_dot_path_rejects_well_root() {
        let (conn, wid, _dir) = setup();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: ".".into(),
                new_path: "anywhere".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::WellRoot));
    }

    #[test]
    fn rename_empty_path_rejects_well_root() {
        let (conn, wid, _dir) = setup();
        let err = folder_rename(
            &conn,
            &wid,
            &FolderRenameInput {
                old_path: "".into(),
                new_path: "anywhere".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, FoldersError::WellRoot));
    }

    // --- folder_move ---

    #[test]
    fn move_is_identical_to_rename_with_different_response_shape() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let resp = folder_move(
            &conn,
            &wid,
            &FolderMoveInput {
                source_path: "src".into(),
                dest_path: "dst".into(),
            },
        )
        .unwrap();
        assert_eq!(resp.source_path, "src");
        assert_eq!(resp.dest_path, "dst");
        assert!(!dir.path().join("src").exists());
        assert!(dir.path().join("dst").is_dir());
    }

    // --- client_message sanitization ---

    #[test]
    fn client_message_sanitizes_internal_variants() {
        let io_err = FoldersError::Io(std::io::Error::other("secret detail"));
        assert_eq!(io_err.client_message(), "internal error");
        let db_err = FoldersError::Db(DbError::from(rusqlite::Error::QueryReturnedNoRows));
        assert_eq!(db_err.client_message(), "internal error");
    }

    #[test]
    fn client_message_exposes_safe_domain_messages() {
        assert_eq!(
            FoldersError::WellNotFound.client_message(),
            "well not found"
        );
        assert_eq!(FoldersError::NotFound.client_message(), "folder not found");
        assert_eq!(
            FoldersError::AlreadyExists.client_message(),
            "folder already exists"
        );
        assert_eq!(
            FoldersError::NotADirectory.client_message(),
            "path is not a directory"
        );
        assert_eq!(
            FoldersError::NotEmpty.client_message(),
            "folder is not empty"
        );
        assert_eq!(
            FoldersError::WellRoot.client_message(),
            "cannot operate on the well root"
        );
        assert_eq!(
            FoldersError::PathSafety(PathSafetyError::Escape).client_message(),
            "invalid path"
        );
    }
}
