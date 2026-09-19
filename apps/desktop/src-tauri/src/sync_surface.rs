//! Filesystem-surface hygiene for `~/.mneme`.
//!
//! Hand-derived from the slice-6 `sync_surface` keeper, UNCHANGED in substance:
//! the pivot did not touch these invariants. We refuse to run inside iCloud /
//! cloud-storage (which corrupts SQLite WAL), drop a `.noindex` marker so
//! Spotlight leaves the DB alone, and ask Time Machine to exclude the dir.
//!
//! The iCloud check and the `.noindex` marker are harmless no-ops outside
//! macOS (the paths/marker they check for simply never apply). Time Machine
//! exclusion (`tmutil`) is macOS-only functionality and is explicitly skipped
//! on other platforms — see [`exclude_from_time_machine`].

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::errors::Error;

pub fn run_all(mneme_dir: &Path) -> Result<(), Error> {
    refuse_if_in_icloud(mneme_dir)?;
    write_noindex_marker(mneme_dir)?;
    exclude_from_time_machine(mneme_dir)?;
    Ok(())
}

fn refuse_if_in_icloud(p: &Path) -> Result<(), Error> {
    let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let s = canon.to_string_lossy();
    if s.contains("/Library/Mobile Documents/") || s.contains("/Library/CloudStorage/") {
        return Err(Error::ICloudRefused(s.into_owned()));
    }
    Ok(())
}

fn write_noindex_marker(p: &Path) -> Result<(), Error> {
    let marker = p.join(".noindex");
    if !marker.exists() {
        fs::File::create(&marker)?;
    }
    Ok(())
}

fn exclude_from_time_machine(p: &Path) -> Result<(), Error> {
    // Time Machine is macOS-only; skip entirely elsewhere rather than spawn a
    // `tmutil` process that's guaranteed to fail (harmlessly, but it would log
    // a confusing warning on every launch on Linux/Windows for nothing).
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    // CRITICAL: `tmutil addexclusion` can take ~10s+ to return, and run_all()
    // executes synchronously inside Tauri's setup() on the MAIN THREAD before the
    // event loop services the window — so a slow addexclusion beachballs the whole
    // app on every launch. Two-layer fix:
    //   1. Skip entirely when the path is already excluded — `tmutil isexcluded`
    //      returns in ~0.1s, and exclusion is sticky, so the common case is fast.
    //   2. When an exclusion IS needed, run addexclusion on a detached background
    //      thread (fire-and-forget) so startup never blocks on it.
    // Exclusion is idempotent and best-effort; a failure is logged, never fatal.
    if is_excluded(p) {
        return Ok(());
    }
    let path = p.to_path_buf();
    std::thread::spawn(move || {
        match Command::new("tmutil")
            .arg("addexclusion")
            .arg(&path)
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => {
                log::warn!(
                "tmutil addexclusion exited with status {status}; backup-drive may not be configured"
            );
            }
            Err(e) => {
                log::warn!("failed to invoke tmutil: {e}");
            }
        }
    });
    Ok(())
}

/// Fast (~0.1s) check via `tmutil isexcluded`: true iff macOS reports the path is
/// already a Time Machine exclusion. Any error / unexpected output → false (we
/// then attempt the exclusion in the background), so we never wrongly skip.
fn is_excluded(p: &Path) -> bool {
    match Command::new("tmutil").arg("isexcluded").arg(p).output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).contains("[Excluded]")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn icloud_path_refused() {
        // We can't create a real iCloud path in CI, so exercise the substring
        // matcher: a dir whose path contains the iCloud marker is refused.
        let tmp = tempdir().unwrap();
        let fake_icloud = tmp
            .path()
            .join("Library")
            .join("Mobile Documents")
            .join("mneme-test");
        fs::create_dir_all(&fake_icloud).unwrap();
        let result = refuse_if_in_icloud(&fake_icloud);
        assert!(matches!(result, Err(Error::ICloudRefused(_))));
    }

    #[test]
    fn normal_path_accepted() {
        let tmp = tempdir().unwrap();
        let normal = tmp.path().join("mneme-test-normal");
        fs::create_dir_all(&normal).unwrap();
        assert!(refuse_if_in_icloud(&normal).is_ok());
    }

    #[test]
    fn noindex_marker_created() {
        let tmp = tempdir().unwrap();
        let marker = tmp.path().join(".noindex");
        assert!(!marker.exists());
        write_noindex_marker(tmp.path()).unwrap();
        assert!(marker.exists());
    }

    #[test]
    fn noindex_marker_idempotent() {
        let tmp = tempdir().unwrap();
        write_noindex_marker(tmp.path()).unwrap();
        // Second call should not error.
        write_noindex_marker(tmp.path()).unwrap();
    }
}
