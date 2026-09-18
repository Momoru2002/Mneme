//! The single local identity ("owner") for this single-user app. Authentication
//! was removed; what remains is one stable identity row that the data schema
//! FK-references (wells.created_by_user_id, active_well.user_id,
//! user_settings.user_id) and that the future git-sharing slice will reuse as the
//! git author + friend-code anchor.

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::services::users::{self, NewUser};

/// Resolve the single owner's `user_id`, creating one passwordless owner row if
/// the users table is empty. Idempotent: on an existing DB (e.g. a dev machine
/// that already ran the old setup flow) it ADOPTS the existing lone user rather
/// than creating a second. The dormant auth columns (password_hash/role) are
/// filled with inert placeholders to satisfy NOT NULL — they are unused now.
pub fn ensure_owner(conn: &Connection) -> Result<String, DbError> {
    if let Some(id) = first_user_id(conn)? {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    users::insert_user(
        conn,
        &NewUser {
            id: id.clone(),
            username: "owner".to_string(),
            password_hash: String::new(), // dormant — no auth
            display_name: "You".to_string(),
            email: None,
            role: crate::rbac::Role::Admin, // dormant — no RBAC
        },
    )?;
    Ok(id)
}

/// The id of the first (oldest) user row, or None if the table is empty.
fn first_user_id(conn: &Connection) -> Result<Option<String>, DbError> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT id FROM users ORDER BY created_at ASC, id ASC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn creates_one_owner_on_empty_db() {
        let conn = open_in_memory().unwrap();
        assert_eq!(users::count_users(&conn).unwrap(), 0);
        let id = ensure_owner(&conn).unwrap();
        assert!(!id.is_empty());
        assert_eq!(users::count_users(&conn).unwrap(), 1, "exactly one owner");
    }

    #[test]
    fn is_idempotent_returns_same_id() {
        let conn = open_in_memory().unwrap();
        let a = ensure_owner(&conn).unwrap();
        let b = ensure_owner(&conn).unwrap();
        assert_eq!(a, b, "second call must adopt, not create");
        assert_eq!(users::count_users(&conn).unwrap(), 1, "still exactly one");
    }

    #[test]
    fn adopts_an_existing_user_row() {
        // Simulates a dev DB already bootstrapped by the old setup flow.
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, display_name, created_at, updated_at) \
             VALUES ('existing-admin', 'alice', 'phc', 'admin', 'Alice', 100, 100)",
            [],
        )
        .unwrap();
        let id = ensure_owner(&conn).unwrap();
        assert_eq!(
            id, "existing-admin",
            "must adopt the existing row, not mint a new one"
        );
        assert_eq!(users::count_users(&conn).unwrap(), 1);
    }

    #[test]
    fn adopts_when_a_row_named_owner_already_exists() {
        // Guards the UNIQUE(username) edge: if a prior row already uses the
        // "owner" username, ensure_owner must ADOPT it (never attempt a second
        // insert that would violate the unique constraint).
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, display_name, created_at, updated_at) \
             VALUES ('pre-owner', 'owner', '', 'admin', 'You', 50, 50)",
            [],
        )
        .unwrap();
        let id = ensure_owner(&conn).unwrap();
        assert_eq!(id, "pre-owner", "must adopt the existing 'owner' row");
        assert_eq!(users::count_users(&conn).unwrap(), 1, "no second insert");
        // A second call is still idempotent.
        assert_eq!(ensure_owner(&conn).unwrap(), "pre-owner");
        assert_eq!(users::count_users(&conn).unwrap(), 1);
    }
}
