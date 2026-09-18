//! SQLite data layer for Mneme (slice 7 native pivot).
//!
//! Decision (2026-05-30 Punakawan, Option A): `rusqlite` (bundled) + `refinery`,
//! with ALL SQL behind this Rust layer — never exposed to the webview. That
//! boundary is the security property the sidecar pivot was paid to buy.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::db::error::DbError;

pub mod error;

/// The application database handle held in Tauri state.
///
/// rusqlite is synchronous and Tauri commands are `async`; for a single-user
/// desktop app the simplest correct bridge is one connection behind a Mutex
/// (2026-05-30 Punakawan condition 1 — design the sync/async seam ONCE). A
/// command handler does `db.0.lock()` and runs its query; contention is a
/// non-issue for one user clicking a UI.
pub struct Db(pub Arc<Mutex<Connection>>);

/// Embedded, version-tracked migrations (refinery). The `migrations/` dir lives
/// at the crate root; each file is `V{n}__{name}.sql`. Checksums are verified at
/// startup, so a tampered or partially-applied migration is detected (Punakawan
/// condition: the audit trail / DB integrity must be trustworthy).
mod embedded {
    refinery::embed_migrations!("migrations");
}

/// Apply the per-connection PRAGMAs Mneme needs, then run all pending migrations.
fn init(conn: &mut Connection) -> Result<(), DbError> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    embedded::migrations::runner().run(conn)?;
    Ok(())
}

/// Open (creating if missing) the on-disk database at `path`, migrated and
/// ready. WAL mode gives durable, crash-safe writes.
pub fn open(path: impl AsRef<std::path::Path>) -> Result<Connection, DbError> {
    let mut conn = Connection::open(path)?;
    // `PRAGMA journal_mode = WAL` RETURNS the mode actually activated. SQLite
    // silently falls back to "delete" when the backing filesystem can't support
    // WAL's shared-memory primitives (some network mounts / sandboxes). Read the
    // result rather than discarding it (D-REV-2) so a degraded durability
    // guarantee is at least visible in the log.
    let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        log::warn!(
            "requested WAL journal mode but SQLite reports '{mode}'; \
             write durability is reduced (the filesystem may not support WAL)"
        );
    }
    init(&mut conn)?;
    Ok(conn)
}

/// Open the application database under `mneme_dir` (normally `~/.mneme`),
/// enforcing the on-disk security boundary (SEC-3): the directory at 0700 and
/// the DB file at 0600. This is the hardened entry point app startup uses.
pub fn open_app_db(mneme_dir: &std::path::Path) -> Result<Connection, Box<dyn std::error::Error>> {
    crate::perms::ensure_mneme_home_at(mneme_dir)?;
    let db_path = mneme_dir.join("mneme.db");
    let conn = open(&db_path)?;
    crate::perms::set_file_0600(&db_path)?;
    Ok(conn)
}

/// Open an in-memory database, migrated and ready. Test-only for now (gated so
/// it doesn't show as dead code in the app build); relax the cfg if a
/// production ephemeral use appears.
#[cfg(test)]
pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    init(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// User tables, excluding sqlite internals and refinery's own history table.
    fn table_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master \
                 WHERE type = 'table' \
                   AND name NOT LIKE 'sqlite_%' \
                   AND name NOT LIKE 'refinery_%' \
                 ORDER BY name",
            )
            .unwrap();
        let names = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        names
    }

    #[test]
    fn migrations_create_the_slice7_tables() {
        let conn = open_in_memory().expect("open in-memory db");
        let tables = table_names(&conn);
        for expected in [
            "active_well",
            "audit_log",
            "user_settings",
            "users",
            "wells",
        ] {
            assert!(
                tables.contains(&expected.to_string()),
                "missing table `{expected}`; got {tables:?}"
            );
        }
    }

    #[test]
    fn sessions_table_is_dropped_in_slice7() {
        // Slice 7 holds the session in-memory (tauri::State), so the cookie-era
        // `sessions` table is intentionally gone.
        let conn = open_in_memory().expect("open in-memory db");
        assert!(
            !table_names(&conn).contains(&"sessions".to_string()),
            "sessions table should not exist in slice 7"
        );
    }

    #[test]
    fn migrations_persist_to_a_real_file_and_do_not_rerun() {
        // Punakawan condition 3: exercise migrations against a real on-disk
        // file (not :memory:), including a reopen — catches file creation, WAL,
        // and refinery's checksum re-validation that an in-memory test hides.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mneme.db");

        {
            let conn = open(&path).expect("open on-disk db");
            assert!(table_names(&conn).contains(&"users".to_string()));
        }
        assert!(path.exists(), "db file should be created on disk");

        {
            let conn = open(&path).expect("reopen on-disk db");
            assert!(table_names(&conn).contains(&"users".to_string()));
            let applied: i64 = conn
                .query_row("SELECT COUNT(*) FROM refinery_schema_history", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(applied, 5, "exactly five migrations recorded after reopen");

            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='well_sync'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "well_sync table exists after migration");

            // V0004: mirror table and kind column must exist.
            let mirror_exists: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='mirror'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(mirror_exists, 1, "mirror table must exist after V0004");

            let kind_exists: bool = conn
                .prepare("PRAGMA table_info(wells)")
                .unwrap()
                .query_map([], |r| r.get::<_, String>(1))
                .unwrap()
                .map(Result::unwrap)
                .any(|col| col == "kind");
            assert!(kind_exists, "wells.kind column must exist after V0004");
        }
    }

    #[test]
    fn v0002_adds_version_history_column_defaulting_off() {
        let conn = open_in_memory().unwrap();
        insert_user(&conn, "u1", "owner");
        conn.execute(
            "INSERT INTO wells (id, name, path, last_accessed_at, created_by_user_id, created_at, updated_at) \
             VALUES ('w1', 'W', '/tmp/w1', 0, 'u1', 0, 0)",
            [],
        )
        .unwrap();
        let enabled: i64 = conn
            .query_row(
                "SELECT version_history_enabled FROM wells WHERE id = 'w1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(enabled, 0, "new wells default to history disabled");
    }

    #[test]
    fn v0005_reconciles_mirror_and_readonly_wells_to_plain_local() {
        // Git features are gone. A pre-existing install may have a well left in a
        // mirror / read-only / version-history state from V0002-V0004. V0005 must
        // reconcile those rows so nothing is stuck in an unserviced state, and the
        // now-orphaned well_sync / mirror rows must be cleared.
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        // Apply only V0001..V0004, i.e. the pre-removal schema, then seed a row in
        // the exact state V0005 is meant to fix.
        embedded::migrations::runner()
            .set_target(refinery::Target::Version(4))
            .run(&mut conn)
            .unwrap();

        insert_user(&conn, "u1", "owner");
        conn.execute(
            "INSERT INTO wells \
             (id, name, path, last_accessed_at, created_by_user_id, created_at, updated_at, \
              kind, read_only, version_history_enabled) \
             VALUES ('m1', 'M', '/tmp/m1', 0, 'u1', 0, 0, 'mirror', 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO well_sync (well_id, remote_url) VALUES ('m1', 'https://example.invalid')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mirror (well_id, remote_url) VALUES ('m1', 'https://example.invalid')",
            [],
        )
        .unwrap();

        // Now run the remaining migration (V0005).
        embedded::migrations::runner().run(&mut conn).unwrap();

        let (kind, read_only, vh): (String, i64, i64) = conn
            .query_row(
                "SELECT kind, read_only, version_history_enabled FROM wells WHERE id = 'm1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(kind, "local", "mirror well must be reconciled to local");
        assert_eq!(read_only, 0, "read_only must be cleared");
        assert_eq!(vh, 0, "version_history_enabled must be cleared");

        let sync_rows: i64 = conn
            .query_row("SELECT count(*) FROM well_sync", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sync_rows, 0, "well_sync rows must be cleared by V0005");
        let mirror_rows: i64 = conn
            .query_row("SELECT count(*) FROM mirror", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mirror_rows, 0, "mirror rows must be cleared by V0005");
    }

    fn insert_user(conn: &Connection, id: &str, username: &str) {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES (?1, ?2, 'hash', ?2, 0, 0)",
            [id, username],
        )
        .unwrap();
    }

    #[test]
    fn wal_mode_is_active_on_disk() {
        // D-REV-2 companion: confirm WAL really activates on a real file (APFS).
        let dir = tempfile::tempdir().unwrap();
        let conn = open(dir.path().join("mneme.db")).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn foreign_keys_are_enforced_rejecting_an_orphan_well() {
        // Proves `PRAGMA foreign_keys = ON` is live: wells.created_by_user_id
        // REFERENCES users(id). Inserting a well for a non-existent user fails.
        let conn = open_in_memory().unwrap();
        let r = conn.execute(
            "INSERT INTO wells (id, name, path, last_accessed_at, created_by_user_id, created_at, updated_at) \
             VALUES ('v1', 'V', '/tmp/v1', 0, 'ghost', 0, 0)",
            [],
        );
        assert!(r.is_err(), "orphan well must violate the foreign key");
    }

    #[test]
    fn unique_index_rejects_duplicate_username() {
        let conn = open_in_memory().unwrap();
        insert_user(&conn, "u1", "alice");
        let r = conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u2', 'alice', 'h', 'A2', 0, 0)",
            [],
        );
        assert!(
            r.is_err(),
            "duplicate username must violate the unique index"
        );
    }

    #[test]
    fn audit_log_user_id_has_no_foreign_key() {
        // Deliberate (V0001 comment): an audit entry must survive a user_id that
        // never existed (or is later deleted), so there is NO FK on this column.
        let conn = open_in_memory().unwrap();
        let r = conn.execute(
            "INSERT INTO audit_log (user_id, action, resource_type, resource_ref, meta, created_at, row_hash) \
             VALUES ('ghost-user', 'login', 'auth', '-', '{}', 0, 'h')",
            [],
        );
        assert!(
            r.is_ok(),
            "audit_log.user_id has no FK; an orphan user_id must be allowed"
        );
    }

    #[test]
    fn open_app_db_enforces_0700_dir_and_0600_file() {
        // SEC-3: the end-to-end on-disk security boundary — the ~/.mneme dir at
        // 0700 and the DB file at 0600, plus a working migrated connection.
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let mneme_dir = tmp.path().join(".mneme");

        let conn = open_app_db(&mneme_dir).expect("open hardened app db");

        assert_eq!(
            std::fs::metadata(&mneme_dir).unwrap().permissions().mode() & 0o777,
            0o700,
            "~/.mneme must be 0700"
        );
        let db_file = mneme_dir.join("mneme.db");
        assert!(db_file.exists());
        assert_eq!(
            std::fs::metadata(&db_file).unwrap().permissions().mode() & 0o777,
            0o600,
            "the db file must be 0600"
        );
        assert!(table_names(&conn).contains(&"users".to_string()));
    }
}
