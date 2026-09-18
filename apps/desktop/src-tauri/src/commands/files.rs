//! File commands — thin `#[tauri::command]` wrappers over `core::files`.
//!
//! Each body acquires the DB lock then delegates to the matching
//! `crate::core::files::*` function.  All business logic, the read-only-mirror
//! guard (`ensure_writable`), the path-confinement guard, and error mapping live
//! in the core layer so they apply identically to the Tauri IPC AND the web-mode
//! HTTP transport — neither path can bypass them.

use tauri::State;

use crate::core::files as core_files;
use crate::db::Db;
use crate::dto::{
    FileContent, FileCreateInput, FileDuplicateInput, FileMoveInput, FileMoveResponse,
    FileRenameInput, FileRenameResponse, FileUpdateInput, SaveFileResponse,
};

use super::lock_db;

#[tauri::command]
pub async fn files_read(
    db: State<'_, Db>,
    well_id: String,
    path: String,
) -> Result<FileContent, String> {
    let conn = lock_db(&db)?;
    core_files::files_read(&conn, &well_id, &path)
}

#[tauri::command]
pub async fn files_create(
    db: State<'_, Db>,
    well_id: String,
    input: FileCreateInput,
) -> Result<SaveFileResponse, String> {
    let conn = lock_db(&db)?;
    core_files::files_create(&conn, &well_id, &input.path, &input.content)
}

#[tauri::command]
pub async fn files_update(
    db: State<'_, Db>,
    well_id: String,
    input: FileUpdateInput,
) -> Result<SaveFileResponse, String> {
    let conn = lock_db(&db)?;
    core_files::files_update(
        &conn,
        &well_id,
        &input.path,
        &input.content,
        input.expected_hash.as_deref(),
    )
}

#[tauri::command]
pub async fn files_remove(db: State<'_, Db>, well_id: String, path: String) -> Result<(), String> {
    let conn = lock_db(&db)?;
    core_files::files_remove(&conn, &well_id, &path)
}

#[tauri::command]
pub async fn files_rename(
    db: State<'_, Db>,
    well_id: String,
    input: FileRenameInput,
) -> Result<FileRenameResponse, String> {
    let conn = lock_db(&db)?;
    core_files::files_rename(&conn, &well_id, &input.old_path, &input.new_path)
}

#[tauri::command]
pub async fn files_move(
    db: State<'_, Db>,
    well_id: String,
    input: FileMoveInput,
) -> Result<FileMoveResponse, String> {
    let conn = lock_db(&db)?;
    core_files::files_move(&conn, &well_id, &input.source_path, &input.dest_path)
}

#[tauri::command]
pub async fn files_duplicate(
    db: State<'_, Db>,
    well_id: String,
    input: FileDuplicateInput,
) -> Result<SaveFileResponse, String> {
    let conn = lock_db(&db)?;
    core_files::files_duplicate(&conn, &well_id, &input.path)
}
