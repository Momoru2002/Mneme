//! Folder commands — thin Tauri wrappers over `core::folders`.
//!
//! Each body acquires the DB lock then delegates to the matching
//! `crate::core::folders::*` function.  Business logic, the read-only-mirror
//! guard, path-confinement, and error mapping live in the core layer so they
//! apply to BOTH the Tauri IPC and the web-mode HTTP transport.

use tauri::State;

use crate::core::folders as core_folders;
use crate::db::Db;
use crate::dto::{
    FolderCreateInput, FolderCreateResponse, FolderMoveInput, FolderMoveResponse,
    FolderRemoveInput, FolderRenameInput, FolderRenameResponse,
};

use super::lock_db;

#[tauri::command]
pub async fn folders_create(
    db: State<'_, Db>,
    well_id: String,
    input: FolderCreateInput,
) -> Result<FolderCreateResponse, String> {
    let conn = lock_db(&db)?;
    core_folders::folders_create(&conn, &well_id, &input)
}

#[tauri::command]
pub async fn folders_remove(
    db: State<'_, Db>,
    well_id: String,
    input: FolderRemoveInput,
) -> Result<(), String> {
    let conn = lock_db(&db)?;
    core_folders::folders_remove(&conn, &well_id, &input)
}

#[tauri::command]
pub async fn folders_rename(
    db: State<'_, Db>,
    well_id: String,
    input: FolderRenameInput,
) -> Result<FolderRenameResponse, String> {
    let conn = lock_db(&db)?;
    core_folders::folders_rename(&conn, &well_id, &input)
}

#[tauri::command]
pub async fn folders_move(
    db: State<'_, Db>,
    well_id: String,
    input: FolderMoveInput,
) -> Result<FolderMoveResponse, String> {
    let conn = lock_db(&db)?;
    core_folders::folders_move(&conn, &well_id, &input)
}
