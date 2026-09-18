//! Transport-agnostic core for settings operations.
//!
//! Each `pub fn` here takes a plain `&Connection` (no Tauri state) and
//! delegates directly to `services::settings` + `services::owner`, mapping
//! errors to sanitized client strings.  Both the Tauri command wrappers and
//! the HTTP handlers call into this module.

use rusqlite::Connection;

use crate::dto::{SettingsResponse, UserPrefsPatch};
use crate::services::{owner, settings};

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn settings_get(conn: &Connection) -> Result<SettingsResponse, String> {
    let user_id = owner::ensure_owner(conn).map_err(|_| "internal error".to_string())?;
    let prefs = settings::get_prefs(conn, &user_id).map_err(|_| "internal error".to_string())?;
    Ok(SettingsResponse { prefs })
}

pub fn settings_update(
    conn: &Connection,
    patch: &UserPrefsPatch,
) -> Result<SettingsResponse, String> {
    let user_id = owner::ensure_owner(conn).map_err(|_| "internal error".to_string())?;
    let prefs = settings::update_prefs(conn, &user_id, patch).map_err(|e| e.client_message())?;
    Ok(SettingsResponse { prefs })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::UserPrefsPatch;

    fn db() -> rusqlite::Connection {
        open_in_memory().unwrap()
    }

    #[test]
    fn settings_get_returns_defaults_on_empty_db() {
        let conn = db();
        let resp = settings_get(&conn).unwrap();
        assert_eq!(resp.prefs.theme, "wellspring-dark");
        assert_eq!(resp.prefs.auto_save_ms, 2000);
    }

    #[test]
    fn settings_update_then_get_roundtrips() {
        let conn = db();
        let patch = UserPrefsPatch {
            font_size: Some(20),
            ..Default::default()
        };
        let resp = settings_update(&conn, &patch).unwrap();
        assert_eq!(resp.prefs.font_size, 20);
        // subsequent get returns the updated value
        let got = settings_get(&conn).unwrap();
        assert_eq!(got.prefs.font_size, 20);
    }

    #[test]
    fn settings_update_rejects_out_of_range() {
        let conn = db();
        let patch = UserPrefsPatch {
            font_size: Some(99),
            ..Default::default()
        };
        let err = settings_update(&conn, &patch).unwrap_err();
        assert!(err.contains("fontSize"), "got: {err}");
    }
}
