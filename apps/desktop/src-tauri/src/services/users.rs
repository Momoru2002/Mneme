//! Data access for the `users` table. Authentication was removed; what remains
//! is the owner-bootstrap repo `services::owner::ensure_owner` uses to create
//! the single owner identity — `insert_user` (+ the `NewUser` struct). The
//! `count_users` helper is test-only (the owner flow uses a raw `SELECT id`,
//! not a count) so it is gated behind `#[cfg(test)]`.

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::rbac::Role;

/// Fields required to create a user.
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: Role,
}

#[cfg(test)]
pub fn count_users(conn: &Connection) -> Result<i64, DbError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?)
}

pub fn insert_user(conn: &Connection, u: &NewUser) -> Result<(), DbError> {
    let now = crate::clock::now_ms();
    conn.execute(
        "INSERT INTO users (id, username, email, password_hash, role, display_name, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        rusqlite::params![
            u.id,
            u.username,
            u.email,
            u.password_hash,
            u.role.as_str(),
            u.display_name,
            now,
        ],
    )?;
    Ok(())
}

/// The owner's current `password_hash` — empty string means "no app-lock
/// password set yet" (the dormant-owner default from `services::owner`).
pub fn get_password_hash(conn: &Connection, user_id: &str) -> Result<String, DbError> {
    Ok(conn.query_row(
        "SELECT password_hash FROM users WHERE id = ?1",
        [user_id],
        |r| r.get(0),
    )?)
}

pub fn set_password_hash(conn: &Connection, user_id: &str, hash: &str) -> Result<(), DbError> {
    let now = crate::clock::now_ms();
    conn.execute(
        "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![hash, now, user_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::rbac::Role;

    #[test]
    fn count_is_zero_on_a_fresh_db() {
        let conn = open_in_memory().unwrap();
        assert_eq!(count_users(&conn).unwrap(), 0);
    }

    #[test]
    fn insert_then_count_round_trips() {
        let conn = open_in_memory().unwrap();
        insert_user(
            &conn,
            &NewUser {
                id: "u1".into(),
                username: "owner".into(),
                password_hash: "".into(),
                display_name: "You".into(),
                email: None,
                role: Role::Admin,
            },
        )
        .unwrap();
        assert_eq!(count_users(&conn).unwrap(), 1);
    }
}
