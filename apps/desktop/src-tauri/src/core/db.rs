//! Transport-agnostic entry point for opening the app database.
//!
//! Everything else in `core/` takes an already-open `&Connection` — this is
//! the one exception, because it's the thing an out-of-process caller (the
//! `mneme-mcp` binary, a separate `[[bin]]` in this same package — see
//! `src/bin/mneme-mcp.rs`) cannot do for itself: `db` and `perms` are private
//! modules (deliberately — they're the on-disk security boundary, see
//! `perms.rs`'s doc comment), so a bin target linking `mneme_lib` as an
//! external crate has no way to reach `perms::mneme_home()` or
//! `db::open_app_db()` directly. This function is the sanctioned, minimal
//! crack in that wall: it does exactly what the desktop app itself does to
//! open its own database, and nothing else.

use rusqlite::Connection;

/// Open the same `~/.mneme/mneme.db` the desktop app uses, with the same
/// directory/file hardening (`perms::ensure_mneme_home_at`,
/// `perms::set_file_0600` — see that module for the Unix/Windows split).
/// SQLite's own locking (this app runs in WAL mode) makes it safe for this to
/// run concurrently with the desktop app already having it open.
pub fn open_app_db() -> Result<Connection, String> {
    let home = crate::perms::mneme_home();
    crate::db::open_app_db(&home).map_err(|e| e.to_string())
}

/// Path for `mneme-mcp`'s own log file — `~/.mneme/mneme-mcp.log`. That
/// binary can't reach `perms::mneme_home()` directly (private module, and
/// `mneme-mcp` links `mneme_lib` as an external crate — see this file's own
/// doc comment), so this is its second sanctioned crack in that wall. Not
/// used by the desktop app itself, which has its own `logging.rs`.
pub fn mcp_log_path() -> std::path::PathBuf {
    crate::perms::mneme_home().join("mneme-mcp.log")
}
