//! Error type for the data layer.

/// Errors surfaced by the SQLite data layer. Maps the underlying `rusqlite` and
/// `refinery` errors; command handlers convert this to a string at the
/// `#[tauri::command]` boundary (the webview never sees SQL internals).
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),
}
