//! Per-user editor/UI preferences, stored as JSON in `user_settings.prefs`.

use rusqlite::{Connection, OptionalExtension};

use crate::clock::now_ms;
use crate::db::error::DbError;
use crate::dto::{UserPrefs, UserPrefsPatch};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("invalid setting: {0}")]
    Invalid(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl SettingsError {
    /// Safe wire message: validation feedback is user-facing; DB internals are not.
    pub fn client_message(&self) -> String {
        match self {
            SettingsError::Invalid(m) => format!("invalid setting: {m}"),
            SettingsError::Db(_) => "internal error".into(),
        }
    }
}

/// Read a user's prefs, falling back to defaults when there is no row (or when a
/// stored partial/older JSON is missing keys — `#[serde(default)]` fills them).
pub fn get_prefs(conn: &Connection, user_id: &str) -> Result<UserPrefs, DbError> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT prefs FROM user_settings WHERE user_id = ?1",
            [user_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match stored {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => UserPrefs::default(),
    })
}

/// Apply a partial patch over the user's current prefs, validate, and upsert.
/// Validation runs BEFORE the write, so an invalid patch persists nothing.
pub fn update_prefs(
    conn: &Connection,
    user_id: &str,
    patch: &UserPrefsPatch,
) -> Result<UserPrefs, SettingsError> {
    let mut prefs = get_prefs(conn, user_id)?;
    if let Some(v) = patch.theme.clone() {
        prefs.theme = v;
    }
    if let Some(v) = patch.auto_save_ms {
        prefs.auto_save_ms = v;
    }
    if let Some(v) = patch.font_size {
        prefs.font_size = v;
    }
    if let Some(v) = patch.tab_width {
        prefs.tab_width = v;
    }
    if let Some(v) = patch.line_numbers {
        prefs.line_numbers = v;
    }
    if let Some(v) = patch.preview_mermaid {
        prefs.preview_mermaid = v;
    }
    if let Some(v) = patch.tree_font_size {
        prefs.tree_font_size = v;
    }
    if let Some(v) = patch.word_wrap {
        prefs.word_wrap = v;
    }
    if let Some(v) = patch.indent_style {
        prefs.indent_style = v;
    }
    if let Some(v) = &patch.accent_color {
        prefs.accent_color = v.clone();
    }
    if let Some(v) = &patch.gold_color {
        prefs.gold_color = v.clone();
    }
    if let Some(v) = patch.allow_network {
        prefs.allow_network = v;
    }
    validate(&prefs)?;

    let json = serde_json::to_string(&prefs).map_err(|e| SettingsError::Invalid(e.to_string()))?;
    conn.execute(
        "INSERT INTO user_settings (user_id, prefs, updated_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT(user_id) DO UPDATE SET prefs = excluded.prefs, updated_at = excluded.updated_at",
        rusqlite::params![user_id, json, now_ms()],
    )
    .map_err(DbError::from)?;
    Ok(prefs)
}

/// Range checks mirroring `userPrefsSchema` (theme is type-safe via the enum).
fn validate(p: &UserPrefs) -> Result<(), SettingsError> {
    if !(500..=60_000).contains(&p.auto_save_ms) {
        return Err(SettingsError::Invalid(
            "autoSaveMs must be 500..=60000".into(),
        ));
    }
    if !(10..=24).contains(&p.font_size) {
        return Err(SettingsError::Invalid("fontSize must be 10..=24".into()));
    }
    if !(2..=8).contains(&p.tab_width) {
        return Err(SettingsError::Invalid("tabWidth must be 2..=8".into()));
    }
    if !(11..=16).contains(&p.tree_font_size) {
        return Err(SettingsError::Invalid(
            "treeFontSize must be 11..=16".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::IndentStyle;

    fn db_with_user() -> (rusqlite::Connection, String) {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u1', 'alice', 'h', 'Alice', 0, 0)",
            [],
        )
        .unwrap();
        (conn, "u1".to_string())
    }

    #[test]
    fn get_prefs_returns_defaults_when_no_row() {
        let (conn, uid) = db_with_user();
        let p = get_prefs(&conn, &uid).unwrap();
        assert_eq!(p, UserPrefs::default());
        assert_eq!(p.theme, "wellspring-dark");
        assert_eq!(p.auto_save_ms, 2000);
    }

    #[test]
    fn update_then_get_round_trips_and_persists() {
        let (conn, uid) = db_with_user();
        let patch = UserPrefsPatch {
            theme: Some("wellspring-light".to_string()),
            font_size: Some(18),
            ..Default::default()
        };
        let updated = update_prefs(&conn, &uid, &patch).unwrap();
        assert_eq!(updated.theme, "wellspring-light");
        assert_eq!(updated.font_size, 18);
        // untouched fields keep their defaults
        assert_eq!(updated.auto_save_ms, 2000);
        assert_eq!(updated.tab_width, 2);
        // persisted across a fresh read
        assert_eq!(get_prefs(&conn, &uid).unwrap(), updated);
    }

    #[test]
    fn update_is_a_partial_merge_not_a_replace() {
        let (conn, uid) = db_with_user();
        update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                font_size: Some(20),
                ..Default::default()
            },
        )
        .unwrap();
        // a later patch touching only theme must NOT reset font_size
        let after = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                theme: Some("sepia".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(after.theme, "sepia");
        assert_eq!(
            after.font_size, 20,
            "earlier change must survive a later partial patch"
        );
    }

    #[test]
    fn update_rejects_out_of_range_and_persists_nothing() {
        let (conn, uid) = db_with_user();
        let r = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                font_size: Some(99),
                ..Default::default()
            },
        );
        assert!(matches!(r, Err(SettingsError::Invalid(_))));
        // validation runs before the write, so nothing was persisted
        assert_eq!(get_prefs(&conn, &uid).unwrap().font_size, 14);
    }

    #[test]
    fn tree_font_size_round_trips_and_validates_range() {
        let (conn, uid) = db_with_user();
        // default is 13
        assert_eq!(get_prefs(&conn, &uid).unwrap().tree_font_size, 13);
        // a valid update persists
        let updated = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                tree_font_size: Some(15),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.tree_font_size, 15);
        assert_eq!(get_prefs(&conn, &uid).unwrap().tree_font_size, 15);
        // out of range is rejected and persists nothing
        let r = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                tree_font_size: Some(20),
                ..Default::default()
            },
        );
        assert!(matches!(r, Err(SettingsError::Invalid(_))));
        assert_eq!(get_prefs(&conn, &uid).unwrap().tree_font_size, 15);
    }

    #[test]
    fn word_wrap_and_indent_style_round_trip_with_defaults() {
        let (conn, uid) = db_with_user();
        let p = get_prefs(&conn, &uid).unwrap();
        assert!(p.word_wrap, "wordWrap defaults true");
        assert_eq!(p.indent_style, IndentStyle::Spaces);
        let updated = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                word_wrap: Some(false),
                indent_style: Some(IndentStyle::Tabs),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!updated.word_wrap);
        assert_eq!(updated.indent_style, IndentStyle::Tabs);
        assert_eq!(get_prefs(&conn, &uid).unwrap(), updated);
    }

    #[test]
    fn accent_color_round_trips_and_clears() {
        let (conn, uid) = db_with_user();
        assert_eq!(get_prefs(&conn, &uid).unwrap().accent_color, None);
        let set = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                accent_color: Some(Some("#fb7185".to_string())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(set.accent_color.as_deref(), Some("#fb7185"));
        assert_eq!(
            get_prefs(&conn, &uid).unwrap().accent_color.as_deref(),
            Some("#fb7185")
        );
        // clear back to None (the double-option path)
        let cleared = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                accent_color: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.accent_color, None);
        // an absent patch leaves it unchanged
        let again = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                accent_color: Some(Some("#34d399".to_string())),
                ..Default::default()
            },
        )
        .unwrap();
        let untouched = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                font_size: Some(16),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(untouched.accent_color.as_deref(), Some("#34d399"));
        let _ = again;
    }

    #[test]
    fn gold_color_round_trips_and_clears() {
        let (conn, uid) = db_with_user();
        assert_eq!(get_prefs(&conn, &uid).unwrap().gold_color, None);
        let set = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                gold_color: Some(Some("#fbbf24".to_string())),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(set.gold_color.as_deref(), Some("#fbbf24"));
        let cleared = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                gold_color: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.gold_color, None);
    }

    #[test]
    fn allow_network_defaults_false_and_round_trips() {
        let (conn, uid) = db_with_user();
        // default is false
        assert!(!get_prefs(&conn, &uid).unwrap().allow_network);
        // can be enabled
        let enabled = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                allow_network: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(enabled.allow_network);
        assert!(get_prefs(&conn, &uid).unwrap().allow_network);
        // can be disabled again
        let disabled = update_prefs(
            &conn,
            &uid,
            &UserPrefsPatch {
                allow_network: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!disabled.allow_network);
        assert!(!get_prefs(&conn, &uid).unwrap().allow_network);
    }

    #[test]
    fn prefs_are_isolated_per_user() {
        // D-AUTHZ-REV: a user's update must not bleed into another user's prefs.
        let (conn, u1) = db_with_user();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u2', 'bob', 'h', 'Bob', 0, 0)",
            [],
        )
        .unwrap();

        update_prefs(
            &conn,
            &u1,
            &UserPrefsPatch {
                font_size: Some(22),
                ..Default::default()
            },
        )
        .unwrap();

        // u2 is untouched — still defaults
        assert_eq!(get_prefs(&conn, "u2").unwrap(), UserPrefs::default());
        assert_eq!(get_prefs(&conn, &u1).unwrap().font_size, 22);
    }
}
