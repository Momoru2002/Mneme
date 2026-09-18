//! Support / diagnostics actions: reveal logs, build a scrubbed diagnostic
//! bundle, and uninstall-and-wipe.
//!
//! Hand-derived from the slice-6 `menu` keeper, ADAPTED to the no-sidecar /
//! no-keychain context:
//!   * `uninstall_and_wipe` no longer purges a Keychain secret (there is none
//!     in slice 7 — the SESSION_SECRET and `security-framework` are gone).
//!   * the diagnostic bundle no longer includes the sidecar log directory
//!     (`~/.mneme/logs`); only the core log dir under `~/Library/Logs` remains.
//!
//! Everything security-relevant is preserved verbatim: the bundle is written to
//! `~/.mneme/diagnostics/` (0700) — never `/tmp` — via mkstemp-style staging
//! that refuses to follow a pre-created symlink, every log line is run through
//! `logging::scrub`, and the final archive is chmod 0600.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::AppHandle;

use crate::errors::Error;
use crate::{logging, perms};

/// Reveal the logs directory in Finder (`open ~/Library/Logs/com.mneme.desktop`).
pub fn reveal_logs(_: AppHandle) -> Result<(), Error> {
    let logs = logging::dirs_logs_dir().join("com.mneme.desktop");
    Command::new("open").arg(logs).status().map_err(Error::Io)?;
    Ok(())
}

/// Open macOS System Settings → Privacy & Security → Files and Folders.
pub fn open_privacy_settings() -> Result<(), Error> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders")
        .status()
        .map_err(Error::Io)?;
    Ok(())
}

/// Build and write a scrubbed diagnostic bundle under `~/.mneme/diagnostics/`.
///
/// Hardening preserved from slice 6:
///   1. Output location `~/.mneme/diagnostics/` (0700), never world-traversable `/tmp`.
///   2. Filename includes the launch UUID (collision-free, traceable to the run).
///   3. mkstemp-style staging inside the diag dir refuses a pre-created symlink.
///   4. Final bundle chmod 0600.
///   5. Every line of every copied log is run through `logging::scrub`.
pub fn copy_diagnostic_bundle(app: AppHandle) -> Result<PathBuf, Error> {
    let launch_uuid = launch_uuid_from_state(&app).unwrap_or_else(|| "no-uuid".to_string());
    let mneme = perms::mneme_home();
    let diag_dir = mneme.join("diagnostics");
    perms::ensure_dir_0700(&diag_dir)?;

    // Slice 7: only the core log dir exists (the sidecar `~/.mneme/logs` is gone).
    let logs_core = logging::dirs_logs_dir().join("com.mneme.desktop");

    build_diagnostic_bundle(&diag_dir, &launch_uuid, &[&logs_core])
}

/// Test-friendly bundle builder: write the bundle into `diag_dir`, named after
/// `launch_uuid`, copying every regular file under each entry of `sources`
/// (line-by-line through `scrub`). Returns the final 0600 bundle path.
pub fn build_diagnostic_bundle(
    diag_dir: &Path,
    launch_uuid: &str,
    sources: &[&Path],
) -> Result<PathBuf, Error> {
    // Stage dir lives inside the diag dir (so the rename step is atomic on the
    // same filesystem) with mkstemp semantics that refuse a pre-created symlink.
    let stage = tempfile::Builder::new()
        .prefix(".mneme-diag-stage-")
        .tempdir_in(diag_dir)
        .map_err(Error::Io)?;
    fs::set_permissions(stage.path(), fs::Permissions::from_mode(0o700)).map_err(Error::Io)?;

    for src in sources {
        if !src.exists() {
            continue;
        }
        copy_dir_scrubbed(
            src,
            &stage.path().join(
                src.file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("logs")),
            ),
        )?;
    }

    // Build the zip via the system `zip` binary (a macOS runtime default;
    // avoids pulling in a Rust zip crate). Run from inside the stage dir so the
    // archive paths are stable + relative.
    let bundle_in_stage = stage.path().join("bundle.zip");
    let status = Command::new("zip")
        .arg("-r")
        .arg(&bundle_in_stage)
        .arg(".")
        .current_dir(stage.path())
        .status()
        .map_err(Error::Io)?;
    if !status.success() {
        return Err(Error::Io(std::io::Error::other(format!(
            "zip exited with status {status}"
        ))));
    }

    fs::set_permissions(&bundle_in_stage, fs::Permissions::from_mode(0o600)).map_err(Error::Io)?;
    let final_path = diag_dir.join(format!("mneme-diagnostic-{launch_uuid}.zip"));
    // Idempotent across repeated invocations for the same launch uuid.
    let _ = fs::remove_file(&final_path);
    fs::rename(&bundle_in_stage, &final_path).map_err(Error::Io)?;
    // Belt-and-suspenders re-chmod after rename.
    fs::set_permissions(&final_path, fs::Permissions::from_mode(0o600)).map_err(Error::Io)?;

    Ok(final_path)
}

fn copy_dir_scrubbed(src: &Path, dst: &Path) -> Result<(), Error> {
    if !src.exists() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(src).map_err(Error::Io)?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        // Don't follow symlinks into the bundle.
        return Ok(());
    }
    if ft.is_dir() {
        fs::create_dir_all(dst).map_err(Error::Io)?;
        fs::set_permissions(dst, fs::Permissions::from_mode(0o700)).map_err(Error::Io)?;
        for entry in fs::read_dir(src).map_err(Error::Io)? {
            let entry = entry.map_err(Error::Io)?;
            copy_dir_scrubbed(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if ft.is_file() {
        let inp = File::open(src).map_err(Error::Io)?;
        let reader = BufReader::new(inp);
        let mut out = File::create(dst).map_err(Error::Io)?;
        fs::set_permissions(dst, fs::Permissions::from_mode(0o600)).map_err(Error::Io)?;
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let scrubbed = logging::scrub(&l);
                    writeln!(out, "{scrubbed}").map_err(Error::Io)?;
                }
                Err(_) => {
                    // A non-UTF-8 (binary) line is dropped — safer than
                    // smuggling raw bytes past the scrubber.
                    writeln!(out, "[scrub: non-utf8 line dropped]").map_err(Error::Io)?;
                }
            }
        }
    }
    Ok(())
}

/// Look up the launch uuid from the Tauri-managed `logging::LogContext`.
fn launch_uuid_from_state(app: &AppHandle) -> Option<String> {
    use tauri::Manager;
    app.try_state::<logging::LogContext>()
        .map(|s| s.launch_uuid.clone())
}

/// Uninstall and wipe: remove `~/.mneme`. Slice 7 has no Keychain secret to
/// purge (the sidecar's SESSION_SECRET is gone). The Time Machine exclusion
/// record remains; the user removes it manually.
pub fn uninstall_and_wipe(_: AppHandle) -> Result<(), Error> {
    let mneme = perms::mneme_home();
    std::fs::remove_dir_all(&mneme)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_log(path: &Path, lines: &[&str]) {
        let mut f = File::create(path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
    }

    #[test]
    fn build_bundle_writes_to_target_dir_and_is_0600() {
        let tmp = tempdir().unwrap();
        let diag_dir = tmp.path().join("diagnostics");
        fs::create_dir_all(&diag_dir).unwrap();
        fs::set_permissions(&diag_dir, fs::Permissions::from_mode(0o700)).unwrap();

        let logs = tmp.path().join("logs");
        fs::create_dir_all(&logs).unwrap();
        write_log(&logs.join("core.log"), &["ok"]);

        let path = build_diagnostic_bundle(&diag_dir, "0190abcd-launch", &[&logs]).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "bundle file must be 0o600, got {mode:o}");
        assert!(path.starts_with(&diag_dir));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("mneme-diagnostic-"));
        assert!(name.ends_with(".zip"));
        assert!(name.contains("0190abcd-launch"));
    }

    #[test]
    fn build_bundle_scrubs_bearer_token_before_zip() {
        let tmp = tempdir().unwrap();
        let diag_dir = tmp.path().join("diagnostics");
        fs::create_dir_all(&diag_dir).unwrap();
        let logs = tmp.path().join("logs");
        fs::create_dir_all(&logs).unwrap();
        write_log(
            &logs.join("requests.log"),
            &[
                "GET /foo HTTP/1.1",
                "Authorization: Bearer ABCsupersecrettoken123",
                "Cookie: id=verysecretcookievalue",
                "X-Other: keepme",
            ],
        );

        let bundle = build_diagnostic_bundle(&diag_dir, "uuid1", &[&logs]).unwrap();

        // `unzip -p` writes all member content to stdout (the zip may deflate,
        // so we can't grep the raw archive bytes).
        let unzipped = Command::new("unzip")
            .arg("-p")
            .arg(&bundle)
            .output()
            .expect("unzip available on macos");
        assert!(unzipped.status.success(), "unzip -p failed");
        let body = String::from_utf8_lossy(&unzipped.stdout).into_owned();

        assert!(
            !body.contains("ABCsupersecrettoken123"),
            "bearer token must be scrubbed from bundle"
        );
        assert!(
            !body.contains("verysecretcookievalue"),
            "cookie value must be scrubbed from bundle"
        );
        assert!(
            body.contains("X-Other"),
            "non-sensitive header should survive"
        );
    }

    #[test]
    fn build_bundle_is_contained_in_caller_diag_dir() {
        let tmp = tempdir().unwrap();
        let diag_dir = tmp.path().join("diagnostics");
        fs::create_dir_all(&diag_dir).unwrap();
        let logs = tmp.path().join("logs");
        fs::create_dir_all(&logs).unwrap();
        write_log(&logs.join("a.log"), &["data"]);

        let bundle = build_diagnostic_bundle(&diag_dir, "uuid2", &[&logs]).unwrap();
        // The bundle must live inside the caller-controlled diag dir — in
        // production `copy_diagnostic_bundle` sets that to `~/.mneme/diagnostics`
        // (0700), never a shared/world-traversable location. We assert
        // containment in `diag_dir` directly: a literal "/tmp" check is
        // unreliable because the test's own tempdir lives under $TMPDIR (often
        // `/tmp/...`), so the real invariant is "under the dir the caller gave".
        assert!(
            bundle.starts_with(&diag_dir),
            "bundle {} must be under the caller's diag dir {}",
            bundle.display(),
            diag_dir.display()
        );
    }

    #[test]
    fn ensure_dir_0700_is_idempotent_for_the_diag_path() {
        let tmp = tempdir().unwrap();
        let diag_dir = tmp.path().join("diagnostics");
        perms::ensure_dir_0700(&diag_dir).unwrap();
        let mode = fs::metadata(&diag_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        perms::ensure_dir_0700(&diag_dir).unwrap();
    }
}
