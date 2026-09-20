//! Tree command — thin Tauri wrapper over `core::tree`.
//!
//! The body acquires the DB lock then delegates to `crate::core::tree::tree_list`.
//! All business logic, path-confinement guards, and error mapping live in the
//! core layer so they can be reused by the HTTP transport without duplication.

use tauri::State;

use crate::auth::AuthState;
use crate::core::tree as core_tree;
use crate::db::Db;
use crate::dto::TreeResponse;

#[tauri::command]
pub async fn tree_list(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    well_id: String,
    path: Option<String>,
) -> Result<TreeResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn =
        db.0.lock()
            .map_err(|_| "database lock poisoned".to_string())?;
    core_tree::tree_list(&conn, &well_id, path.as_deref().unwrap_or(""))
}
