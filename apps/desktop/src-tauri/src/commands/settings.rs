//! Settings commands — thin Tauri wrappers over `core::settings`.
//!
//! Each body acquires the DB lock then delegates to the matching
//! `crate::core::settings::*` function.  All business logic and error mapping
//! live in the core layer so they can be reused by the HTTP transport without
//! duplication.

use tauri::State;

use crate::core::settings as core_settings;
use crate::db::Db;
use crate::dto::{SettingsResponse, UserPrefsPatch};

use super::lock_db;

#[tauri::command]
pub async fn settings_get(db: State<'_, Db>) -> Result<SettingsResponse, String> {
    let conn = lock_db(&db)?;
    core_settings::settings_get(&conn)
}

#[tauri::command]
pub async fn settings_update(
    db: State<'_, Db>,
    patch: UserPrefsPatch,
) -> Result<SettingsResponse, String> {
    let conn = lock_db(&db)?;
    core_settings::settings_update(&conn, &patch)
}
