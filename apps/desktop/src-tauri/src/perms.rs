//! The `~/.mneme` app-private-data-directory invariant — the on-disk security
//! boundary (verdict: "the DB file is the on-disk security boundary").
//!
//! Two implementations, split by what each OS can actually enforce:
//!
//! - **Unix** (`imp::unix`, ported by hand from the slice-6 perms keeper):
//!   POSIX mode bits give a precise, checkable invariant — 0700 on the
//!   directory, 0600 on the DB file, refuse (don't silently fix) if the
//!   owning uid is wrong or a world bit is set, since either is a compromise
//!   indicator.
//! - **Windows** (`imp::windows`): there is no mode-bit/uid equivalent. NTFS
//!   already restricts a user's profile directory (where `~/.mneme` lives —
//!   see [`mneme_home`]) to that user plus Administrators/SYSTEM by default,
//!   inherited automatically on every file and subdirectory created under
//!   it — so this implementation creates the directory and lets that
//!   inheritance do the work, rather than hand-rolling a DACL/SID call I
//!   have no Windows machine to verify against. This is **best-effort
//!   parity, not an identical guarantee**: unlike the Unix path, it does not
//!   detect or refuse an already-compromised (e.g. manually-shared) profile
//!   directory. Tightening this to an explicit "owner-only" ACL, mirroring
//!   the Unix refuse-on-drift behavior, is a real follow-up if Mneme is ever
//!   used on a shared/multi-admin Windows machine.

use std::fs;
use std::path::PathBuf;

use crate::errors::Error;

/// `~/.mneme` (Unix: `$HOME/.mneme`) or `%USERPROFILE%\.mneme` (Windows) — the
/// app's private data directory. Uses the `dirs` crate rather than reading
/// `$HOME` directly so this also resolves correctly on Windows, where the
/// equivalent variable is `%USERPROFILE%`, not `%HOME%`.
pub fn mneme_home() -> PathBuf {
    let home = dirs::home_dir().expect("could not determine the user's home directory");
    home.join(".mneme")
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::Path;

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
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::path::Path;

    /// Create `dir` if it doesn't exist. No mode bits to set — relies on NTFS
    /// inheriting the surrounding profile directory's ACL (see module doc).
    pub fn ensure_mneme_home_at(dir: &Path) -> Result<(), Error> {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Create `dir` (and parents) if missing. Same rationale as
    /// [`ensure_mneme_home_at`] — no mode bits on Windows.
    pub fn ensure_dir_0700(dir: &Path) -> Result<(), Error> {
        fs::create_dir_all(dir)?;
        Ok(())
    }

    /// No-op on Windows: there is no 0600-equivalent single bit to set, and the
    /// file already inherits its parent directory's (profile-restricted) ACL.
    pub fn set_file_0600(_p: &Path) -> Result<(), Error> {
        Ok(())
    }
}

pub use imp::{ensure_dir_0700, ensure_mneme_home_at, set_file_0600};

#[cfg(test)]
#[cfg(unix)]
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

#[cfg(test)]
#[cfg(windows)]
mod windows_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ensure_home_creates_nonexistent_dir() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join(".mneme");
        ensure_mneme_home_at(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn ensure_home_is_idempotent_on_existing_dir() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join(".mneme");
        ensure_mneme_home_at(&path).unwrap();
        ensure_mneme_home_at(&path).unwrap(); // must not error the second time
        assert!(path.exists());
    }

    #[test]
    fn set_file_0600_is_a_harmless_noop() {
        let tmp = tempdir().unwrap();
        let f = tmp.path().join("mneme.db");
        std::fs::write(&f, b"x").unwrap();
        set_file_0600(&f).unwrap();
        assert!(f.exists());
    }
}
