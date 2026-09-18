//! File-tree listing under a Well — one level (lazy), markdown + folders only.
//! Ports the slice-6 tree-service: skips hidden entries, shows only `.md` files
//! and directories, folders-first sort. Every path is confined to the Well root
//! via `path_safety::resolve_well_path` (the Well root is canonical — a sound
//! anchor). Symlinked entries are not traversed (listed via lstat file-type).

use std::path::Path;
use std::time::UNIX_EPOCH;

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::dto::{TreeEntry, TreeResponse};
use crate::path_safety::{resolve_well_path, PathSafetyError};
use crate::services::wells;

const MD_EXT: &str = ".md";

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("well or path not found")]
    NotFound,
    #[error("invalid path")]
    PathSafety(#[from] PathSafetyError),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl TreeError {
    pub fn client_message(&self) -> String {
        match self {
            TreeError::NotFound => "well or path not found".into(),
            TreeError::PathSafety(_) => "invalid path".into(),
            TreeError::Db(_) => "internal error".into(),
        }
    }
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A directory is "expandable" if it holds a non-hidden subfolder or `.md` file.
fn dir_has_md_or_folders(abs: &Path) -> bool {
    std::fs::read_dir(abs)
        .map(|rd| {
            rd.flatten().any(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if is_hidden(&name) {
                    return false;
                }
                match e.file_type() {
                    Ok(ft) if ft.is_dir() => true,
                    Ok(ft) if ft.is_file() && name.ends_with(MD_EXT) => true,
                    _ => false,
                }
            })
        })
        .unwrap_or(false)
}

pub fn tree_list(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
) -> Result<TreeResponse, TreeError> {
    let well = wells::get(conn, well_id)?.ok_or(TreeError::NotFound)?;
    let requested = if rel_path.is_empty() { "." } else { rel_path };
    let abs_dir = resolve_well_path(&well.path, requested)?;
    let rd = std::fs::read_dir(&abs_dir).map_err(|_| TreeError::NotFound)?;

    let mut entries: Vec<TreeEntry> = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) {
            continue;
        }
        // file_type() is lstat-based: a symlink is neither is_dir nor is_file,
        // so symlinked entries are skipped (never traversed).
        let ft = match e.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let (is_dir, is_file) = (ft.is_dir(), ft.is_file());
        if is_file && !name.ends_with(MD_EXT) {
            continue;
        }
        if !is_dir && !is_file {
            continue;
        }

        let child_abs = e.path();
        let child_rel = if rel_path.is_empty() {
            name.clone()
        } else {
            format!("{rel_path}/{name}")
        };

        let mut size = None;
        let mut mtime = 0;
        let mut has_children = false;
        if let Ok(meta) = std::fs::metadata(&child_abs) {
            mtime = mtime_ms(&meta);
            if is_dir {
                has_children = dir_has_md_or_folders(&child_abs);
            } else {
                size = Some(meta.len() as i64);
            }
        }

        entries.push(TreeEntry {
            name,
            path: child_rel,
            kind: if is_dir {
                "folder".into()
            } else {
                "file".into()
            },
            size,
            mtime,
            has_children,
        });
    }

    // Folders first, then files; each group sorted by name.
    entries.sort_by(|a, b| match (a.kind.as_str(), b.kind.as_str()) {
        ("folder", "file") => std::cmp::Ordering::Less,
        ("file", "folder") => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(TreeResponse {
        well_id: well_id.to_string(),
        path: rel_path.to_string(),
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::AddWellInput;

    /// Create a user + a Well rooted at a populated tempdir; return (conn, well_id, tempdir).
    fn well_with_files() -> (Connection, String, tempfile::TempDir) {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u1', 'alice', 'h', 'Alice', 0, 0)",
            [],
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.md"), b"# b").unwrap();
        std::fs::write(dir.path().join("a.md"), b"# a").unwrap();
        std::fs::write(dir.path().join("note.txt"), b"x").unwrap(); // non-md, skipped
        std::fs::write(dir.path().join(".hidden.md"), b"x").unwrap(); // hidden, skipped
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("c.md"), b"# c").unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();

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

    #[test]
    fn lists_md_and_folders_sorted_folders_first() {
        let (conn, id, _dir) = well_with_files();
        let resp = tree_list(&conn, &id, "").unwrap();
        let names: Vec<_> = resp
            .entries
            .iter()
            .map(|e| (e.kind.as_str(), e.name.as_str()))
            .collect();
        // folders first (empty, sub), then md files (a, b); .txt + hidden excluded
        assert_eq!(
            names,
            vec![
                ("folder", "empty"),
                ("folder", "sub"),
                ("file", "a.md"),
                ("file", "b.md")
            ]
        );
        let sub = resp.entries.iter().find(|e| e.name == "sub").unwrap();
        assert!(sub.has_children, "sub holds c.md");
        let empty = resp.entries.iter().find(|e| e.name == "empty").unwrap();
        assert!(!empty.has_children);
        let a = resp.entries.iter().find(|e| e.name == "a.md").unwrap();
        assert_eq!(a.size, Some(3));
    }

    #[test]
    fn lists_a_subdirectory_by_relative_path() {
        let (conn, id, _dir) = well_with_files();
        let resp = tree_list(&conn, &id, "sub").unwrap();
        assert_eq!(resp.path, "sub");
        let names: Vec<_> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, vec!["sub/c.md"]);
    }

    #[test]
    fn rejects_traversal_outside_the_well() {
        let (conn, id, _dir) = well_with_files();
        assert!(matches!(
            tree_list(&conn, &id, "../.."),
            Err(TreeError::PathSafety(_))
        ));
    }

    #[test]
    fn unknown_well_is_not_found() {
        let (conn, _id, _dir) = well_with_files();
        assert!(matches!(
            tree_list(&conn, "ghost", ""),
            Err(TreeError::NotFound)
        ));
    }
}
