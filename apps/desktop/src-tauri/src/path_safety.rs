//! Intra-well path confinement. Resolves a requested path (relative to a Well
//! root, or absolute) and guarantees the result stays at or inside the Well —
//! rejecting traversal (`..`), absolute escapes, null bytes, and symlink
//! escapes. The Well root is stored canonical (see `services::wells::add`), so
//! it is a sound trust anchor. Used by the Tree / Files / Folders modules.
//!
//! Ported from the slice-6 `apps/api/src/lib/path-safety.ts` `resolveWellPath`,
//! minus the Docker `toContainerPath` mapping (native = direct host fs).

use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathSafetyError {
    #[error("path contains a null byte")]
    NullByte,
    #[error("well path must be absolute")]
    NotAbsolute,
    #[error("path escapes the well")]
    Escape,
    #[error("symlink escapes the well")]
    SymlinkEscape,
}

/// Lexically normalize a path — resolve `.` and `..` purely by string/component
/// logic, WITHOUT touching the filesystem (so `..` cannot be smuggled past the
/// containment check before the fs is consulted).
fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Resolve `requested` against the canonical absolute `well_path`, returning an
/// absolute path guaranteed to sit at or inside the Well. `requested` may be
/// relative to the Well root or absolute; either way it must not escape.
pub fn resolve_well_path(well_path: &str, requested: &str) -> Result<PathBuf, PathSafetyError> {
    if requested.contains('\0') {
        return Err(PathSafetyError::NullByte);
    }
    let well = Path::new(well_path);
    if !well.is_absolute() {
        return Err(PathSafetyError::NotAbsolute);
    }

    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        well.join(requested)
    };
    let resolved = lexical_normalize(&candidate);

    // Component-wise containment (not string-prefix): `resolved` must be the
    // Well root or sit beneath it.
    if !resolved.starts_with(well) {
        return Err(PathSafetyError::Escape);
    }

    // Symlink-escape defense (D-P8SEC-1). Canonicalize the deepest EXISTING
    // ancestor of `resolved` (possibly `resolved` itself) and require its real
    // (link-resolved) location to stay within the canonical well root. Walking
    // to the deepest *existing* ancestor — rather than only checking when the
    // full target already exists — means the guard ALSO holds for a write whose
    // final target does not exist yet (file_create, and the destinations of
    // rename / move / duplicate, folder_create): a symlinked intermediate
    // directory component planted inside the well would otherwise let the write
    // follow the link out of the well. Lexical containment above already blocks
    // `..` / absolute escapes; this closes the remaining symlinked-component hole.
    let well_real = std::fs::canonicalize(well).unwrap_or_else(|_| well.to_path_buf());
    let mut probe: &Path = resolved.as_path();
    let existing = loop {
        if probe.exists() {
            break probe;
        }
        match probe.parent() {
            // `resolved` is lexically contained in `well` (checked above) and
            // `well` exists, so this terminates at or above the well root.
            Some(parent) => probe = parent,
            None => return Err(PathSafetyError::Escape),
        }
    };
    let real = std::fs::canonicalize(existing).map_err(|_| PathSafetyError::Escape)?;
    if real != well_real && !real.starts_with(&well_real) {
        return Err(PathSafetyError::SymlinkEscape);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_relative_path_within_the_well() {
        let dir = tempfile::tempdir().unwrap();
        let well = std::fs::canonicalize(dir.path()).unwrap();
        let well_s = well.to_string_lossy();
        let r = resolve_well_path(&well_s, "notes/a.md").unwrap();
        assert_eq!(r, well.join("notes/a.md"));
        // "." resolves to the well root itself
        assert_eq!(resolve_well_path(&well_s, ".").unwrap(), well);
        assert_eq!(resolve_well_path(&well_s, "").unwrap(), well);
    }

    #[test]
    fn rejects_parent_traversal_escape() {
        let dir = tempfile::tempdir().unwrap();
        let well = std::fs::canonicalize(dir.path()).unwrap();
        let well_s = well.to_string_lossy().into_owned();
        assert_eq!(
            resolve_well_path(&well_s, "../outside"),
            Err(PathSafetyError::Escape)
        );
        assert_eq!(
            resolve_well_path(&well_s, "a/b/../../../escape"),
            Err(PathSafetyError::Escape)
        );
    }

    #[test]
    fn rejects_absolute_path_outside_the_well() {
        let dir = tempfile::tempdir().unwrap();
        let well_s = std::fs::canonicalize(dir.path())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            resolve_well_path(&well_s, "/etc/passwd"),
            Err(PathSafetyError::Escape)
        );
    }

    #[test]
    fn rejects_null_byte_and_relative_well() {
        assert_eq!(
            resolve_well_path("/well", "a\0b"),
            Err(PathSafetyError::NullByte)
        );
        assert_eq!(
            resolve_well_path("relative/well", "a"),
            Err(PathSafetyError::NotAbsolute)
        );
    }

    #[test]
    fn rejects_symlink_that_escapes_the_well() {
        let well_dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let well = std::fs::canonicalize(well_dir.path()).unwrap();
        // well/link -> outside (a symlink pointing out of the well)
        std::os::unix::fs::symlink(outside.path(), well.join("link")).unwrap();
        let err = resolve_well_path(&well.to_string_lossy(), "link").unwrap_err();
        assert_eq!(err, PathSafetyError::SymlinkEscape);
    }

    #[test]
    fn rejects_create_through_a_symlinked_dir_when_target_does_not_exist() {
        // D-P8SEC-1 regression: the symlink-escape guard must hold for a WRITE
        // whose final target does NOT exist yet (file_create / rename-dest /
        // duplicate-dest / folder_create). A symlinked intermediate directory
        // component inside the well (e.g. an imported/shared vault) would
        // otherwise let the write follow the link out of the well.
        let well_dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let well = std::fs::canonicalize(well_dir.path()).unwrap();
        // well/escape -> outside (an intra-well symlink to an existing dir)
        std::os::unix::fs::symlink(outside.path(), well.join("escape")).unwrap();
        // `newfile.md` does NOT exist yet — this is the create path.
        let err = resolve_well_path(&well.to_string_lossy(), "escape/newfile.md").unwrap_err();
        assert_eq!(
            err,
            PathSafetyError::SymlinkEscape,
            "create through a symlinked dir must be rejected even when the \
             target file does not exist yet"
        );
    }

    #[test]
    fn allows_create_of_a_not_yet_existing_path_within_the_well() {
        // The hardened guard must NOT over-reject a legitimate create whose
        // parent dirs don't exist yet (create_dir_all makes them).
        let well_dir = tempfile::tempdir().unwrap();
        let well = std::fs::canonicalize(well_dir.path()).unwrap();
        let got = resolve_well_path(&well.to_string_lossy(), "new/nested/dir/file.md").unwrap();
        assert_eq!(got, well.join("new/nested/dir/file.md"));
    }
}
