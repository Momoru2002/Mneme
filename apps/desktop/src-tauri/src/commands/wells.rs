//! Well commands; errors sanitized.  list/get/activate delegate to `core::wells`;
//! add/update/remove/validate/browse_host/host_home remain direct service calls
//! since they are not in scope for HTTP dispatch.

use tauri::State;

use crate::auth::AuthState;
use crate::core::wells as core_wells;
use crate::db::Db;
use crate::dto::{
    AddWellInput, HostBrowseResponse, UpdateWellInput, Well, WellListResponse, WellValidationResult,
};
use crate::services::{owner, wells};

use super::lock_db;

#[tauri::command]
pub async fn wells_list(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<WellListResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    core_wells::wells_list(&conn)
}

#[tauri::command]
pub async fn wells_get(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    id: String,
) -> Result<Well, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    core_wells::wells_get(&conn, &id)
}

#[tauri::command]
pub async fn wells_add(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    input: AddWellInput,
) -> Result<Well, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    let user_id = owner::ensure_owner(&conn).map_err(|_| "internal error".to_string())?;
    wells::add(&conn, &input, &user_id).map_err(|e| e.client_message())
}

#[tauri::command]
pub async fn wells_update(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    id: String,
    input: UpdateWellInput,
) -> Result<Well, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    wells::update(&conn, &id, &input).map_err(|e| e.client_message())
}

#[tauri::command]
pub async fn wells_remove(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    id: String,
) -> Result<(), String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    wells::remove(&conn, &id).map_err(|e| e.client_message())
}

#[tauri::command]
pub async fn wells_activate(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    id: String,
) -> Result<(), String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = lock_db(&db)?;
    core_wells::wells_activate(&conn, &id)
}

#[tauri::command]
pub async fn wells_validate(
    auth: State<'_, std::sync::Arc<AuthState>>,
    path: String,
) -> Result<WellValidationResult, String> {
    crate::auth::require_unlocked(&auth)?;
    Ok(wells::validate_well_path(&path))
}

#[tauri::command]
pub async fn wells_browse_host(
    auth: State<'_, std::sync::Arc<AuthState>>,
    path: String,
) -> Result<HostBrowseResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    wells::browse_host(&path).map_err(|e| e.client_message())
}

#[tauri::command]
pub async fn wells_host_home(auth: State<'_, std::sync::Arc<AuthState>>) -> Result<HostBrowseResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    wells::host_home().map_err(|e| e.client_message())
}
