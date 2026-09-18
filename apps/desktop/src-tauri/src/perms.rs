//! The `~/.mneme` 0700 + DB-file 0600 invariant — the on-disk security
//! boundary (verdict: "the DB file at 0o600 is the on-disk security boundary").
//!
//! Ported by hand from the slice-6 perms keeper, KEEPING the directory 0700
//! policy and DROPPING the sidecar-era helpers that no longer have callers:
//! `verify_dir_invariant_strict` (the per-request UDS re-stat — no socket now)
//! and `ensure_dir_0700` (the diagnostics-bundle dir — returns with the `menu`
//! keeper). The file helper is now a SETTER (`set_file_0600`) rather than the
//! slice-6 strict verifier: slice 7 is the sole producer of the DB file, so
//! re-asserting 0600 on open (narrowing is always safe) is simpler and more
//! robust than refusing drift across a process boundary that no longer exists.
//!
//! macOS / unix only; a `cfg(windows)` path is a slice-8 cross-platform concern.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::errors::Error;

/// `~/.mneme` — the app's private data directory.
pub fn mneme_home() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".mneme")
}

/// Ensure `dir` exists at mode 0700, owned by the current uid. Creates it if
/// missing; auto-repairs benign group bits (a user who manually chmod'd);
/// REFUSES any world bit or a wrong owner (compromise indicators — surface,
/// don't silently "fix").
pub fn ensure_mneme_home_at(dir: &Path) -> Result<(), Error> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        return Ok(());
    }
    let meta = fs::metadata(dir)?;
    let mode = meta.mode() & 0o777;
    if meta.uid() != unsafe { libc::getuid() } {
        return Err(Error::Perms {
            path: dir.display().to_string(),
            expected: 0o700,
            actual: mode,
            hint: Some("directory not owned by the current user; refusing".into()),
        });
    }
    // World bit set => compromise indicator => refuse (do not auto-repair).
    if mode & 0o007 != 0 {
        return Err(Error::Perms {
            path: dir.display().to_string(),
            expected: 0o700,
            actual: mode,
            hint: Some("world-accessible ~/.mneme; refusing".into()),
        });
    }
    // Benign drift (group bits, no world bits) => auto-repair to 0700.
    if mode != 0o700 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Ensure `dir` exists at mode 0700, creating it (and parents) if missing.
/// Unlike [`ensure_mneme_home_at`], this is a plain create-or-narrow used for
/// app-private subdirectories (e.g. the diagnostics-bundle staging dir — the
/// `menu` keeper's caller); it does not refuse on drift, it just narrows.
/// Restored with the `menu` keeper (slice 7).
pub fn ensure_dir_0700(dir: &Path) -> Result<(), Error> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

/// Force `p` to mode 0600 (owner read/write only). Narrowing is always safe, so
/// this re-asserts the invariant on every open rather than refusing drift — the
/// slice-7 app is the sole producer of this file. Refuses only on a wrong owner.
pub fn set_file_0600(p: &Path) -> Result<(), Error> {
    let meta = fs::metadata(p)?;
    if meta.uid() != unsafe { libc::getuid() } {
        return Err(Error::Perms {
            path: p.display().to_string(),
            expected: 0o600,
            actual: meta.mode() & 0o777,
            hint: Some(format!("owner uid mismatch on {}; refusing", p.display())),
        });
    }
    fs::set_permissions(p, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Error;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn make_dir(parent: &Path, mode: u32) -> PathBuf {
        let p = parent.join(".mneme");
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(mode)).unwrap();
        p
    }

    #[test]
    fn ensure_home_creates_nonexistent_at_0700() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join(".mneme");
        ensure_mneme_home_at(&path).unwrap();
        assert!(path.exists());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn ensure_home_refuses_world_accessible() {
        let tmp = tempdir().unwrap();
        let path = make_dir(tmp.path(), 0o707);
        assert!(matches!(
            ensure_mneme_home_at(&path).unwrap_err(),
            Error::Perms { .. }
        ));
    }

    #[test]
    fn ensure_home_auto_repairs_benign_group_bits() {
        let tmp = tempdir().unwrap();
        let path = make_dir(tmp.path(), 0o750);
        ensure_mneme_home_at(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn ensure_home_is_idempotent_on_already_0700() {
        let tmp = tempdir().unwrap();
        let path = make_dir(tmp.path(), 0o700);
        ensure_mneme_home_at(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn set_file_0600_narrows_world_readable() {
        let tmp = tempdir().unwrap();
        let f = tmp.path().join("mneme.db");
        fs::write(&f, b"x").unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o644)).unwrap();
        set_file_0600(&f).unwrap();
        assert_eq!(
            fs::metadata(&f).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn set_file_0600_is_idempotent_on_already_secure() {
        let tmp = tempdir().unwrap();
        let f = tmp.path().join("mneme.db");
        fs::write(&f, b"x").unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o600)).unwrap();
        set_file_0600(&f).unwrap();
        assert_eq!(
            fs::metadata(&f).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn ensure_dir_0700_creates_and_is_idempotent() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("diagnostics");
        ensure_dir_0700(&dir).unwrap();
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        // Re-run on a pre-existing (looser) dir narrows it back to 0700.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        ensure_dir_0700(&dir).unwrap();
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
