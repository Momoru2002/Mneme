//! Crate-wide "shell" error type (filesystem / permission concerns).
//!
//! Ported by hand from the slice-6 errors keeper, dropping every
//! sidecar-era variant (SidecarHash, Keychain, LivenessFailed,
//! ReadinessTimeout) — those primitives are gone with the pivot. The data
//! layer keeps its own `db::error::DbError`; this is for the app shell.
//!
//! `ICloudRefused` is restored with the `sync_surface` keeper (slice 7): the
//! iCloud-refusal invariant survives the pivot — `~/.mneme` must not live inside
//! a cloud-synced directory (which corrupts SQLite WAL) regardless of whether a
//! sidecar exists.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    // Optional `hint` text appended to the message so a surfaced launch dialog
    // can be actionable (e.g. "run `chmod 600 <path>`") instead of leaving the
    // user staring at an octal mode.
    //
    // Only ever constructed by perms.rs's #[cfg(unix)] path (mode-bit / uid
    // checks have no Windows equivalent — see that module's doc comment), so
    // it is genuinely dead code in a Windows *library* build specifically
    // (unlike the test binary, which does construct it in errors.rs's own
    // tests below, unconditionally — clippy's dead-code check is per
    // compilation unit, and the plain lib target doesn't include test code).
    #[cfg_attr(windows, allow(dead_code))]
    #[error("perms: expected mode {expected:o} on {path}, found {actual:o}{}", format_hint(.hint))]
    Perms {
        path: String,
        expected: u32,
        actual: u32,
        hint: Option<String>,
    },
    // `~/.mneme` resolved to a path inside iCloud Drive / a cloud-sync folder.
    // Cloud sync corrupts SQLite WAL files, so we refuse to run there
    // (restored with the `sync_surface` keeper).
    #[error("icloud refusal: ~/.mneme is inside a cloud-synced directory at {0}")]
    ICloudRefused(String),
}

/// Render the optional remediation hint as a trailing `". <text>"`, or empty.
/// A free function (not a method) so the thiserror `#[error(...)]` format
/// string above can reference it.
fn format_hint(hint: &Option<String>) -> String {
    match hint {
        Some(h) if !h.is_empty() => format!(". {h}"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perms_error_appends_hint_when_present() {
        let e = Error::Perms {
            path: "/x/mneme.db".into(),
            expected: 0o600,
            actual: 0o644,
            hint: Some("run `chmod 600 /x/mneme.db`".into()),
        };
        let msg = e.to_string();
        assert!(msg.contains("600"), "{msg}");
        assert!(msg.contains("644"), "{msg}");
        assert!(msg.contains("chmod 600"), "hint should be appended: {msg}");
    }

    #[test]
    fn perms_error_without_hint_has_no_trailing_segment() {
        let e = Error::Perms {
            path: "/x".into(),
            expected: 0o700,
            actual: 0o755,
            hint: None,
        };
        assert!(
            !e.to_string().contains(". "),
            "no hint => no trailing segment"
        );
    }
}
