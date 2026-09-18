//! Daily SQLite snapshot rotation + integrity check.
//!
//! Hand-derived from the slice-6 `snapshots` keeper, ADAPTED to the slice-7 data
//! layer. Slice 6 shelled out to the system `sqlite3` binary (`VACUUM INTO` and
//! `PRAGMA integrity_check` as subprocesses). Slice 7 bundles `rusqlite`, so the
//! external binary is gone — a user's Mac may not even have `sqlite3` on PATH.
//! Both operations now run through the live `rusqlite::Connection`:
//!
//!   * `VACUUM INTO ?1` with a **bound parameter** — which also retires the
//!     slice-6 single-quote-injection guard (no SQL string is built by hand).
//!   * `PRAGMA integrity_check` read directly off the connection.
//!
//! Snapshotting stays best-effort: a VACUUM failure logs a warning and is
//! non-fatal (it must never block app start); only filesystem errors propagate.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::errors::Error;

const MAX_SNAPSHOTS: usize = 7;
const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Take a `VACUUM INTO` snapshot of the live DB if the newest existing snapshot
/// is at least a day old (or none exists). Best-effort: a VACUUM failure is
/// logged and swallowed so it can never block startup.
pub fn snapshot_if_due(conn: &Connection, snapshots_dir: &Path) -> Result<(), Error> {
    fs::create_dir_all(snapshots_dir)?;
    let last = latest_snapshot_age(snapshots_dir).unwrap_or(Duration::MAX);
    if last < ONE_DAY {
        return Ok(());
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let target = snapshots_dir.join(format!("mneme-{ts}.db"));
    let target_str = target.to_string_lossy().into_owned();

    // VACUUM INTO with a bound parameter — no hand-built SQL string, so the
    // slice-6 quote-injection guard is unnecessary. Best-effort: on failure we
    // log and return Ok (matches the slice-6 "log + non-fatal" contract).
    if let Err(e) = conn.execute("VACUUM INTO ?1", rusqlite::params![target_str]) {
        log::warn!("snapshot VACUUM INTO failed for {target_str}: {e}");
        return Ok(());
    }
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600))?;
    rotate(snapshots_dir, MAX_SNAPSHOTS)?;
    Ok(())
}

fn latest_snapshot_age(dir: &Path) -> Option<Duration> {
    let mut newest: Option<SystemTime> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let meta = entry.metadata().ok()?;
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().ok()?;
        match newest {
            Some(prev) if mtime <= prev => {}
            _ => newest = Some(mtime),
        }
    }
    newest.and_then(|t| SystemTime::now().duration_since(t).ok())
}

fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?;
            Some((e.path(), t))
        })
        .collect();
    entries.sort_by_key(|(_, t)| *t);
    while entries.len() > keep {
        let (p, _) = entries.remove(0);
        let _ = fs::remove_file(p);
    }
    Ok(())
}

/// Run `PRAGMA integrity_check` on the live connection. Returns `true` iff
/// SQLite reports the single row `"ok"`.
pub fn integrity_check(conn: &Connection) -> Result<bool, DbError> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?;
    Ok(result.trim() == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch(p: &std::path::Path, age_seconds: u64) {
        std::fs::write(p, b"").unwrap();
        let when = SystemTime::now() - Duration::from_secs(age_seconds);
        let when_libc = libc::timeval {
            tv_sec: when.duration_since(UNIX_EPOCH).unwrap().as_secs() as libc::time_t,
            tv_usec: 0,
        };
        let times = [when_libc, when_libc];
        let cpath = std::ffi::CString::new(p.to_string_lossy().as_bytes()).unwrap();
        unsafe {
            libc::utimes(cpath.as_ptr(), times.as_ptr());
        }
    }

    /// A minimal real SQLite DB via the bundled rusqlite (no external binary).
    fn make_db(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute("CREATE TABLE t (id INTEGER)", []).unwrap();
        conn
    }

    #[test]
    fn rotation_keeps_max() {
        let tmp = tempdir().unwrap();
        for i in 0..10 {
            touch(&tmp.path().join(format!("mneme-{i}.db")), (10 - i) * 100);
        }
        rotate(tmp.path(), MAX_SNAPSHOTS).unwrap();
        assert_eq!(
            std::fs::read_dir(tmp.path()).unwrap().count(),
            MAX_SNAPSHOTS
        );
    }

    #[test]
    fn integrity_check_on_valid_sqlite() {
        let tmp = tempdir().unwrap();
        let conn = make_db(&tmp.path().join("test.db"));
        assert!(integrity_check(&conn).unwrap());
    }

    #[test]
    fn snapshot_creates_file_and_due_logic_works() {
        let tmp = tempdir().unwrap();
        let conn = make_db(&tmp.path().join("test.db"));
        let snaps = tmp.path().join("snapshots");

        // First snapshot: created.
        snapshot_if_due(&conn, &snaps).unwrap();
        assert_eq!(std::fs::read_dir(&snaps).unwrap().count(), 1);

        // Second snapshot immediately: SKIPPED (under ONE_DAY).
        snapshot_if_due(&conn, &snaps).unwrap();
        assert_eq!(std::fs::read_dir(&snaps).unwrap().count(), 1);
    }

    #[test]
    fn snapshot_file_is_0600_and_is_a_valid_db() {
        let tmp = tempdir().unwrap();
        let conn = make_db(&tmp.path().join("test.db"));
        let snaps = tmp.path().join("snapshots");
        snapshot_if_due(&conn, &snaps).unwrap();

        let snap = std::fs::read_dir(&snaps)
            .unwrap()
            .flatten()
            .next()
            .unwrap()
            .path();
        // 0600 on the snapshot file.
        let mode = std::fs::metadata(&snap).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "snapshot must be 0600, got {mode:o}");
        // And it is itself a valid SQLite DB that passes integrity_check.
        let snap_conn = Connection::open(&snap).unwrap();
        assert!(integrity_check(&snap_conn).unwrap());
    }
}
