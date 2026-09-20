//! Search command — thin `#[tauri::command]` wrapper over `core::search`.
//!
//! The query-length cap, path confinement (in the service), and error mapping
//! live in the core layer so they apply identically to the Tauri IPC AND the
//! web-mode HTTP transport.

use tauri::State;

use crate::auth::AuthState;
use crate::core::search as core_search;
use crate::db::Db;
use crate::dto::{SearchOptions, SearchResponse};

/// Tauri command: search `well_id` for `q` (case-insensitive substring).
///
/// `include_content` — when `Some(false)` only filename matches are returned;
/// defaults to `true` (mirrors `content !== 'false'` in the slice-6 route).
#[tauri::command]
pub async fn search_query(
    db: State<'_, Db>,
    auth: State<'_, std::sync::Arc<AuthState>>,
    well_id: String,
    q: String,
    include_content: Option<bool>,
    options: Option<SearchOptions>,
) -> Result<SearchResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    let conn = crate::commands::lock_db(&db)?;
    core_search::search_query(
        &conn,
        &well_id,
        &q,
        include_content.unwrap_or(true),
        &options.unwrap_or_default(),
    )
}
