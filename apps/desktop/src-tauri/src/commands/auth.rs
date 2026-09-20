//! Tauri commands for the app-lock (see `auth.rs` for the hashing/state
//! logic this wraps). Deliberately NOT gated by `auth::require_unlocked`
//! itself — you obviously need to call `auth_unlock` while still locked.

use tauri::State;

use crate::auth::AuthState;
use crate::db::Db;
use crate::services::{owner, users};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    /// Whether a password has ever been set. `false` means first-run: the UI
    /// should show "create a password" rather than "enter your password".
    pub is_set_up: bool,
    pub unlocked: bool,
}

#[tauri::command]
pub async fn auth_status(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<AuthStatus, String> {
    let conn = db.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let owner_id = owner::ensure_owner(&conn).map_err(|e| e.to_string())?;
    let hash = users::get_password_hash(&conn, &owner_id).map_err(|e| e.to_string())?;
    Ok(AuthStatus {
        is_set_up: !hash.is_empty(),
        unlocked: auth.is_unlocked(),
    })
}

/// First-run only: set the app-lock password. Refuses if one is already set
/// — use `auth_change_password` to change an existing one (that requires
/// proving you know the current password first).
#[tauri::command]
pub async fn auth_set_password(
    password: String,
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<(), String> {
    if password.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    let conn = db.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let owner_id = owner::ensure_owner(&conn).map_err(|e| e.to_string())?;
    let existing = users::get_password_hash(&conn, &owner_id).map_err(|e| e.to_string())?;
    if !existing.is_empty() {
        return Err("a password is already set".to_string());
    }
    let hash = crate::auth::hash_password(&password)?;
    users::set_password_hash(&conn, &owner_id, &hash).map_err(|e| e.to_string())?;
    auth.set_unlocked(true);
    Ok(())
}

#[tauri::command]
pub async fn auth_unlock(
    password: String,
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let owner_id = owner::ensure_owner(&conn).map_err(|e| e.to_string())?;
    let hash = users::get_password_hash(&conn, &owner_id).map_err(|e| e.to_string())?;
    drop(conn);
    if hash.is_empty() {
        // Nothing set up yet — nothing to unlock against.
        return Err("no password is set".to_string());
    }
    if crate::auth::verify_password(&password, &hash)? {
        auth.set_unlocked(true);
        Ok(())
    } else {
        Err("incorrect password".to_string())
    }
}

/// Re-lock immediately without closing the app (e.g. a "Lock now" menu item).
#[tauri::command]
pub async fn auth_lock(auth: State<'_, std::sync::Arc<AuthState>>) -> Result<(), String> {
    auth.set_unlocked(false);
    Ok(())
}

#[tauri::command]
pub async fn auth_change_password(
    current_password: String,
    new_password: String,
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<(), String> {
    crate::auth::require_unlocked(&auth)?;
    if new_password.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    let conn = db.0.lock().map_err(|_| "db lock poisoned".to_string())?;
    let owner_id = owner::ensure_owner(&conn).map_err(|e| e.to_string())?;
    let hash = users::get_password_hash(&conn, &owner_id).map_err(|e| e.to_string())?;
    if hash.is_empty() || !crate::auth::verify_password(&current_password, &hash)? {
        return Err("current password is incorrect".to_string());
    }
    let new_hash = crate::auth::hash_password(&new_password)?;
    users::set_password_hash(&conn, &owner_id, &new_hash).map_err(|e| e.to_string())?;
    Ok(())
}
