//! File operations inside a Well: read / create / update / remove / rename /
//! move / duplicate — markdown (.md) only.
//!
//! Every path (source AND destination) is resolved through
//! `path_safety::resolve_well_path` against the Well's canonical root before
//! any filesystem operation. A traversal on a write, move, or delete is the
//! worst-case scenario and is blocked at the resolution step.
//!
//! Ports `apps/api/src/services/file-service.ts` 1:1 (adapting Node/Postgres
//! idioms to Rust + std::fs). Frontmatter parsing mirrors `gray-matter`:
//! if the file starts with `---\n`, the YAML block up to the closing `---` is
//! extracted; otherwise `frontmatter` is an empty map and `body == content`.
//!
//! Hash is SHA-256 hex of the raw file content (frontmatter NOT stripped),
//! matching the slice-6 contract.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::db::error::DbError;
use crate::dto::{FileContent, FileMoveResponse, FileRenameResponse, SaveFileResponse};
use crate::path_safety::{resolve_well_path, PathSafetyError};
use crate::services::wells;

const MD_EXT: &str = ".md";

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum FilesError {
    #[error("well not found")]
    WellNotFound,
    #[error("file not found")]
    NotFound,
    #[error("only .md files are supported")]
    NotMarkdown,
    #[error("path is a directory")]
    IsDirectory,
    #[error("file already exists")]
    AlreadyExists,
    #[error("file was changed by another process")]
    HashMismatch,
    #[error("invalid path")]
    PathSafety(#[from] PathSafetyError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

impl FilesError {
    pub fn client_message(&self) -> String {
        match self {
            FilesError::WellNotFound => "well not found".into(),
            FilesError::NotFound => "file not found".into(),
            FilesError::NotMarkdown => "only .md files are supported".into(),
            FilesError::IsDirectory => "path is a directory".into(),
            FilesError::AlreadyExists => "file already exists".into(),
            FilesError::HashMismatch => "file was changed by another process".into(),
            FilesError::PathSafety(_) => "invalid path".into(),
            FilesError::Db(_) | FilesError::Io(_) => "internal error".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// SHA-256 hex digest of UTF-8 content — matches `sha256()` in file-service.ts.
fn sha256(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}

/// Resolve `rel_path` inside the well — the single path-confinement chokepoint.
fn resolve(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
) -> Result<(String, PathBuf), FilesError> {
    let well = wells::get(conn, well_id)?.ok_or(FilesError::WellNotFound)?;
    let abs = resolve_well_path(&well.path, rel_path)?;
    Ok((well.path, abs))
}

fn mtime_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn file_size(path: &Path) -> i64 {
    std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0)
}

/// Minimal frontmatter parser that mirrors gray-matter's default behavior.
/// If the file starts with `---\n`, everything up to the next `---` line is
/// treated as the YAML header; the remainder is the body. We do not interpret
/// the YAML — we store it as raw key-value strings (matching the TypeScript
/// type `Frontmatter = Record<string, unknown>`). For the desktop this is
/// sufficient; actual rendering happens in the webview.
pub(crate) fn parse_frontmatter(content: &str) -> (String, HashMap<String, serde_json::Value>) {
    // Find the closing `---` line
    let after_open = if let Some(rest) = content.strip_prefix("---\r\n") {
        rest
    } else if let Some(rest) = content.strip_prefix("---\n") {
        rest
    } else {
        return (content.to_string(), HashMap::new());
    };
    // Look for `\n---` or `\r\n---` followed by end-of-string or newline
    let close = after_open
        .find("\n---\n")
        .or_else(|| after_open.find("\n---\r\n"))
        .or_else(|| {
            if after_open.ends_with("\n---") {
                Some(after_open.len() - 4)
            } else {
                None
            }
        });

    match close {
        None => (content.to_string(), HashMap::new()),
        Some(idx) => {
            let yaml_src = &after_open[..idx];
            // Parse each `key: value` line (best-effort; complex YAML is stored verbatim)
            let mut fm: HashMap<String, serde_json::Value> = HashMap::new();
            for line in yaml_src.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim().to_string();
                    let val = v.trim();
                    // Attempt JSON-compatible type inference (number / bool / string)
                    let jv: serde_json::Value = if let Ok(n) = val.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else if val == "true" {
                        serde_json::Value::Bool(true)
                    } else if val == "false" {
                        serde_json::Value::Bool(false)
                    } else {
                        serde_json::Value::String(val.to_string())
                    };
                    if !key.is_empty() {
                        fm.insert(key, jv);
                    }
                }
            }
            // Body starts after the closing `---` delimiter
            let after_close_idx = idx + 1; // skip \n before ---
            let body_start = after_open[after_close_idx..]
                .find('\n')
                .map(|i| after_close_idx + i + 1)
                .unwrap_or(after_open.len());
            let body = after_open[body_start..].to_string();
            (body, fm)
        }
    }
}

// ---------------------------------------------------------------------------
// Operations (port of file-service.ts)
// ---------------------------------------------------------------------------

/// Read a markdown file, returning its content, parsed body, frontmatter,
/// size, mtime, and SHA-256 hash. Ports `readMarkdownFile`.
pub fn file_read(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
) -> Result<FileContent, FilesError> {
    if !rel_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    let (_, abs) = resolve(conn, well_id, rel_path)?;
    if !abs.exists() {
        return Err(FilesError::NotFound);
    }
    if abs.is_dir() {
        return Err(FilesError::IsDirectory);
    }
    let content = std::fs::read_to_string(&abs)?;
    let (body, frontmatter) = parse_frontmatter(&content);
    let size = file_size(&abs);
    let mtime = mtime_ms(&abs);
    let hash = sha256(&content);
    Ok(FileContent {
        well_id: well_id.to_string(),
        path: rel_path.to_string(),
        content,
        body,
        frontmatter,
        size,
        mtime,
        hash,
    })
}

/// Create a new markdown file (parent dirs are created recursively).
/// Ports `createFile`.
pub fn file_create(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
    content: &str,
) -> Result<SaveFileResponse, FilesError> {
    if !rel_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    let (_, abs) = resolve(conn, well_id, rel_path)?;
    if abs.exists() {
        return Err(FilesError::AlreadyExists);
    }
    // create_dir_all is safe here: the parent is guaranteed inside the well
    // because `abs` was resolved via resolve_well_path.
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&abs, content.as_bytes())?;
    let size = file_size(&abs);
    let mtime = mtime_ms(&abs);
    Ok(SaveFileResponse {
        path: rel_path.to_string(),
        size,
        mtime,
        hash: sha256(content),
    })
}

/// Overwrite an existing markdown file, with optional optimistic-concurrency
/// guard (expectedHash). Ports `updateFile`.
pub fn file_update(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
    content: &str,
    expected_hash: Option<&str>,
) -> Result<SaveFileResponse, FilesError> {
    if !rel_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    let (_, abs) = resolve(conn, well_id, rel_path)?;
    if !abs.exists() {
        return Err(FilesError::NotFound);
    }
    if abs.is_dir() {
        return Err(FilesError::IsDirectory);
    }
    if let Some(expected) = expected_hash {
        let current = std::fs::read_to_string(&abs)?;
        if sha256(&current) != expected {
            return Err(FilesError::HashMismatch);
        }
    }
    std::fs::write(&abs, content.as_bytes())?;
    let size = file_size(&abs);
    let mtime = mtime_ms(&abs);
    Ok(SaveFileResponse {
        path: rel_path.to_string(),
        size,
        mtime,
        hash: sha256(content),
    })
}

/// Delete a markdown file. Ports `deleteFile`.
pub fn file_remove(conn: &Connection, well_id: &str, rel_path: &str) -> Result<(), FilesError> {
    if !rel_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    let (_, abs) = resolve(conn, well_id, rel_path)?;
    if !abs.exists() {
        return Err(FilesError::NotFound);
    }
    if abs.is_dir() {
        return Err(FilesError::IsDirectory);
    }
    std::fs::remove_file(&abs)?;
    Ok(())
}

/// Rename a markdown file (old_path → new_path, both resolved through
/// path_safety). Parent dirs of new_path are created if needed.
/// Ports `renameFile`.
pub fn file_rename(
    conn: &Connection,
    well_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<FileRenameResponse, FilesError> {
    if !new_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    // Resolve BOTH paths — traversal on either side is rejected.
    let well = wells::get(conn, well_id)?.ok_or(FilesError::WellNotFound)?;
    let abs_old = resolve_well_path(&well.path, old_path)?;
    let abs_new = resolve_well_path(&well.path, new_path)?;

    if !abs_old.exists() {
        return Err(FilesError::NotFound);
    }
    if abs_new.exists() {
        return Err(FilesError::AlreadyExists);
    }
    if let Some(parent) = abs_new.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::rename(&abs_old, &abs_new)?;
    Ok(FileRenameResponse {
        old_path: old_path.to_string(),
        new_path: new_path.to_string(),
    })
}

/// Move a markdown file (source_path → dest_path). Same semantics as rename;
/// separate command for API clarity. Ports `moveFile`.
pub fn file_move(
    conn: &Connection,
    well_id: &str,
    source_path: &str,
    dest_path: &str,
) -> Result<FileMoveResponse, FilesError> {
    file_rename(conn, well_id, source_path, dest_path)?;
    Ok(FileMoveResponse {
        source_path: source_path.to_string(),
        dest_path: dest_path.to_string(),
    })
}

/// Duplicate a markdown file next to the original, using a "(copy)" / "(copy N)"
/// suffix pattern. Ports `duplicateFile`.
pub fn file_duplicate(
    conn: &Connection,
    well_id: &str,
    rel_path: &str,
) -> Result<SaveFileResponse, FilesError> {
    if !rel_path.ends_with(MD_EXT) {
        return Err(FilesError::NotMarkdown);
    }
    let well = wells::get(conn, well_id)?.ok_or(FilesError::WellNotFound)?;
    let abs_src = resolve_well_path(&well.path, rel_path)?;
    if !abs_src.exists() {
        return Err(FilesError::NotFound);
    }

    // Compute candidate names in the same directory as the source.
    let dir = Path::new(rel_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base = Path::new(rel_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let join_rel = |d: &str, name: &str| -> String {
        if d.is_empty() || d == "." {
            name.to_string()
        } else {
            format!("{d}/{name}")
        }
    };

    let mut candidate_name = format!("{base} (copy){MD_EXT}");
    let mut suffix = 1u32;
    loop {
        let candidate_rel = join_rel(&dir, &candidate_name);
        let candidate_abs = resolve_well_path(&well.path, &candidate_rel)?;
        if !candidate_abs.exists() {
            // Found a free slot.
            std::fs::copy(&abs_src, &candidate_abs)?;
            let content = std::fs::read_to_string(&candidate_abs)?;
            let size = file_size(&candidate_abs);
            let mtime = mtime_ms(&candidate_abs);
            return Ok(SaveFileResponse {
                path: candidate_rel,
                size,
                mtime,
                hash: sha256(&content),
            });
        }
        suffix += 1;
        candidate_name = format!("{base} (copy {suffix}){MD_EXT}");
    }
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

    // --- sha256 ---

    #[test]
    fn sha256_is_hex_string() {
        let h = sha256("hello");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // --- parse_frontmatter ---

    #[test]
    fn parse_frontmatter_no_yaml() {
        let (body, fm) = parse_frontmatter("# Hello\n\nworld");
        assert_eq!(body, "# Hello\n\nworld");
        assert!(fm.is_empty());
    }

    #[test]
    fn parse_frontmatter_with_yaml() {
        let src = "---\ntitle: My Note\ntags: rust\n---\n# Body\n";
        let (body, fm) = parse_frontmatter(src);
        assert_eq!(body, "# Body\n");
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("My Note"));
        assert_eq!(fm.get("tags").and_then(|v| v.as_str()), Some("rust"));
    }

    #[test]
    fn parse_frontmatter_numeric_and_bool() {
        let src = "---\ncount: 42\ndraft: true\n---\nbody\n";
        let (body, fm) = parse_frontmatter(src);
        assert_eq!(body, "body\n");
        assert_eq!(fm.get("count").and_then(|v| v.as_i64()), Some(42));
        assert_eq!(fm.get("draft").and_then(|v| v.as_bool()), Some(true));
    }

    // --- file_read ---

    #[test]
    fn read_returns_content_and_metadata() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"# Hello\n").unwrap();
        let fc = file_read(&conn, &wid, "note.md").unwrap();
        assert_eq!(fc.path, "note.md");
        assert_eq!(fc.content, "# Hello\n");
        assert_eq!(fc.hash, sha256("# Hello\n"));
        assert!(fc.size > 0);
        assert!(fc.mtime > 0);
    }

    #[test]
    fn read_parses_frontmatter() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("fm.md"), b"---\ntitle: T\n---\nbody\n").unwrap();
        let fc = file_read(&conn, &wid, "fm.md").unwrap();
        assert_eq!(fc.body, "body\n");
        assert_eq!(
            fc.frontmatter.get("title").and_then(|v| v.as_str()),
            Some("T")
        );
    }

    #[test]
    fn read_rejects_non_md() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_read(&conn, &wid, "note.txt"),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn read_not_found() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_read(&conn, &wid, "ghost.md"),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn read_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_read(&conn, &wid, "../escape.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    #[test]
    fn read_is_directory_error() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("sub.md")).unwrap(); // dir named .md
        assert!(matches!(
            file_read(&conn, &wid, "sub.md"),
            Err(FilesError::IsDirectory)
        ));
    }

    #[test]
    fn read_unknown_well() {
        let conn = open_in_memory().unwrap();
        assert!(matches!(
            file_read(&conn, "ghost", "a.md"),
            Err(FilesError::WellNotFound)
        ));
    }

    // --- file_create ---

    #[test]
    fn create_writes_file() {
        let (conn, wid, dir) = setup();
        let resp = file_create(&conn, &wid, "new.md", "hello").unwrap();
        assert_eq!(resp.path, "new.md");
        assert_eq!(resp.hash, sha256("hello"));
        assert!(dir.path().join("new.md").exists());
    }

    #[test]
    fn create_makes_parent_dirs() {
        let (conn, wid, dir) = setup();
        file_create(&conn, &wid, "a/b/note.md", "").unwrap();
        assert!(dir.path().join("a/b/note.md").exists());
    }

    #[test]
    fn create_rejects_already_exists() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();
        assert!(matches!(
            file_create(&conn, &wid, "exists.md", ""),
            Err(FilesError::AlreadyExists)
        ));
    }

    #[test]
    fn create_rejects_non_md() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_create(&conn, &wid, "doc.txt", ""),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn create_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_create(&conn, &wid, "../escape.md", ""),
            Err(FilesError::PathSafety(_))
        ));
    }

    // --- file_update ---

    #[test]
    fn update_overwrites_content() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"old").unwrap();
        let resp = file_update(&conn, &wid, "note.md", "new", None).unwrap();
        assert_eq!(resp.hash, sha256("new"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("note.md")).unwrap(),
            "new"
        );
    }

    #[test]
    fn update_with_correct_hash_succeeds() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"original").unwrap();
        let h = sha256("original");
        file_update(&conn, &wid, "note.md", "updated", Some(&h)).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("note.md")).unwrap(),
            "updated"
        );
    }

    #[test]
    fn update_with_wrong_hash_is_hash_mismatch() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"current").unwrap();
        assert!(matches!(
            file_update(&conn, &wid, "note.md", "new", Some("badhash")),
            Err(FilesError::HashMismatch)
        ));
    }

    #[test]
    fn update_not_found() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_update(&conn, &wid, "ghost.md", "x", None),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn update_rejects_non_md() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_update(&conn, &wid, "doc.txt", "x", None),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn update_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_update(&conn, &wid, "../escape.md", "x", None),
            Err(FilesError::PathSafety(_))
        ));
    }

    // --- file_remove ---

    #[test]
    fn remove_deletes_file() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("todelete.md"), b"x").unwrap();
        file_remove(&conn, &wid, "todelete.md").unwrap();
        assert!(!dir.path().join("todelete.md").exists());
    }

    #[test]
    fn remove_not_found() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_remove(&conn, &wid, "ghost.md"),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn remove_rejects_non_md() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_remove(&conn, &wid, "doc.txt"),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn remove_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_remove(&conn, &wid, "../escape.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    #[test]
    fn remove_directory_path_is_error() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("dir.md")).unwrap();
        assert!(matches!(
            file_remove(&conn, &wid, "dir.md"),
            Err(FilesError::IsDirectory)
        ));
    }

    // --- file_rename ---

    #[test]
    fn rename_moves_and_renames_file() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("old.md"), b"x").unwrap();
        let resp = file_rename(&conn, &wid, "old.md", "new.md").unwrap();
        assert_eq!(resp.old_path, "old.md");
        assert_eq!(resp.new_path, "new.md");
        assert!(!dir.path().join("old.md").exists());
        assert!(dir.path().join("new.md").exists());
    }

    #[test]
    fn rename_creates_parent_dirs() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("src.md"), b"x").unwrap();
        file_rename(&conn, &wid, "src.md", "sub/dst.md").unwrap();
        assert!(dir.path().join("sub/dst.md").exists());
    }

    #[test]
    fn rename_rejects_non_md_destination() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"x").unwrap();
        assert!(matches!(
            file_rename(&conn, &wid, "note.md", "note.txt"),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn rename_source_not_found() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_rename(&conn, &wid, "ghost.md", "new.md"),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn rename_dest_already_exists() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("a.md"), b"x").unwrap();
        std::fs::write(dir.path().join("b.md"), b"y").unwrap();
        assert!(matches!(
            file_rename(&conn, &wid, "a.md", "b.md"),
            Err(FilesError::AlreadyExists)
        ));
    }

    #[test]
    fn rename_rejects_traversal_on_source() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_rename(&conn, &wid, "../escape.md", "new.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    #[test]
    fn rename_rejects_traversal_on_destination() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("src.md"), b"x").unwrap();
        assert!(matches!(
            file_rename(&conn, &wid, "src.md", "../escape.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    // --- file_move ---

    #[test]
    fn move_is_rename_with_different_response() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("src.md"), b"x").unwrap();
        let resp = file_move(&conn, &wid, "src.md", "dst.md").unwrap();
        assert_eq!(resp.source_path, "src.md");
        assert_eq!(resp.dest_path, "dst.md");
        assert!(!dir.path().join("src.md").exists());
        assert!(dir.path().join("dst.md").exists());
    }

    #[test]
    fn move_rejects_traversal_on_dest() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("src.md"), b"x").unwrap();
        assert!(matches!(
            file_move(&conn, &wid, "src.md", "../escape.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    // --- file_duplicate ---

    #[test]
    fn duplicate_creates_copy_next_to_original() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"content").unwrap();
        let resp = file_duplicate(&conn, &wid, "note.md").unwrap();
        assert_eq!(resp.path, "note (copy).md");
        assert!(dir.path().join("note (copy).md").exists());
        assert_eq!(resp.hash, sha256("content"));
    }

    #[test]
    fn duplicate_avoids_collision_with_suffix() {
        let (conn, wid, dir) = setup();
        std::fs::write(dir.path().join("note.md"), b"x").unwrap();
        std::fs::write(dir.path().join("note (copy).md"), b"x").unwrap();
        let resp = file_duplicate(&conn, &wid, "note.md").unwrap();
        assert_eq!(resp.path, "note (copy 2).md");
    }

    #[test]
    fn duplicate_in_subdirectory() {
        let (conn, wid, dir) = setup();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/note.md"), b"x").unwrap();
        let resp = file_duplicate(&conn, &wid, "sub/note.md").unwrap();
        assert_eq!(resp.path, "sub/note (copy).md");
        assert!(dir.path().join("sub/note (copy).md").exists());
    }

    #[test]
    fn duplicate_not_found() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_duplicate(&conn, &wid, "ghost.md"),
            Err(FilesError::NotFound)
        ));
    }

    #[test]
    fn duplicate_rejects_non_md() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_duplicate(&conn, &wid, "doc.txt"),
            Err(FilesError::NotMarkdown)
        ));
    }

    #[test]
    fn duplicate_rejects_traversal() {
        let (conn, wid, _dir) = setup();
        assert!(matches!(
            file_duplicate(&conn, &wid, "../escape.md"),
            Err(FilesError::PathSafety(_))
        ));
    }

    // --- client_message sanitization ---

    #[test]
    fn client_message_sanitizes_internal_variants() {
        let io_err = FilesError::Io(std::io::Error::other("secret"));
        assert_eq!(io_err.client_message(), "internal error");
        let db_err = FilesError::Db(DbError::from(rusqlite::Error::QueryReturnedNoRows));
        assert_eq!(db_err.client_message(), "internal error");
    }

    #[test]
    fn client_message_exposes_safe_domain_messages() {
        assert_eq!(FilesError::WellNotFound.client_message(), "well not found");
        assert_eq!(FilesError::NotFound.client_message(), "file not found");
        assert_eq!(
            FilesError::NotMarkdown.client_message(),
            "only .md files are supported"
        );
        assert_eq!(
            FilesError::IsDirectory.client_message(),
            "path is a directory"
        );
        assert_eq!(
            FilesError::AlreadyExists.client_message(),
            "file already exists"
        );
        assert_eq!(
            FilesError::HashMismatch.client_message(),
            "file was changed by another process"
        );
        assert_eq!(
            FilesError::PathSafety(PathSafetyError::Escape).client_message(),
            "invalid path"
        );
    }
}
