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
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::AppHandle;

use crate::errors::Error;
use crate::{logging, perms};

/// Open a path or URL with the OS's default handler — `open` on macOS,
/// `cmd /C start` on Windows (invoking the shell's `start` builtin directly
/// isn't possible; `start` isn't its own executable), `xdg-open` elsewhere
/// (Linux). (Not used for `open_privacy_settings`, which invokes a
/// macOS-only System Settings URL scheme and has no equivalent elsewhere.)
pub(crate) fn open_with_os_default(target: impl AsRef<std::ffi::OsStr>) -> Result<(), Error> {
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(target).status()
    } else if cfg!(target_os = "windows") {
        // The leading "" is a deliberate no-op window-title argument: `start`
        // treats the first quoted argument as a title, so a target path that
        // itself contains spaces/quotes would otherwise be misparsed as one.
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(target)
            .status()
    } else {
        Command::new("xdg-open").arg(target).status()
    };
    status.map_err(Error::Io)?;
    Ok(())
}

/// Reveal the logs directory in the OS file manager (Finder on macOS,
/// Explorer on Windows, whatever `xdg-open` resolves to on Linux).
pub fn reveal_logs(_: AppHandle) -> Result<(), Error> {
    let logs = logging::dirs_logs_dir().join("com.mneme.desktop");
    open_with_os_default(logs)
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
    perms::ensure_dir_0700(stage.path())?;

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

    // Build the zip: the system `zip` binary on macOS/Linux (a runtime
    // default on both; avoids pulling in a Rust zip crate), PowerShell's
    // `Compress-Archive` on Windows (bundled since Windows 10 — no `zip.exe`
    // ships with Windows). The in-progress archive is written to `diag_dir`
    // (a SIBLING of `stage`), not inside `stage` itself: `Compress-Archive`
    // refuses to write a destination that lives inside the tree it's
    // compressing (macOS/Linux `zip` tolerates this; PowerShell does not),
    // and avoiding it is simply more correct on every platform regardless.
    let bundle_tmp = diag_dir.join(format!(".mneme-diag-{launch_uuid}.zip"));
    zip_directory(stage.path(), &bundle_tmp)?;

    perms::set_file_0600(&bundle_tmp)?;
    let final_path = diag_dir.join(format!("mneme-diagnostic-{launch_uuid}.zip"));
    // Idempotent across repeated invocations for the same launch uuid.
    let _ = fs::remove_file(&final_path);
    fs::rename(&bundle_tmp, &final_path).map_err(Error::Io)?;
    // Belt-and-suspenders re-chmod after rename.
    perms::set_file_0600(&final_path)?;

    Ok(final_path)
}

/// Zip everything under `src_dir` into `zip_path`. See [`build_diagnostic_bundle`]
/// for why this shells out rather than using a Rust zip crate, and why the
/// command differs by OS.
fn zip_directory(src_dir: &Path, zip_path: &Path) -> Result<(), Error> {
    let status = if cfg!(target_os = "windows") {
        // Compress-Archive refuses to overwrite without -Force, and wants a
        // glob for "everything in this folder". Single-quote the paths and
        // double any embedded single quote (PowerShell's escape for a
        // single-quoted string) since these paths come from the user's own
        // profile directory name.
        let ps_escape = |p: &Path| p.display().to_string().replace('\'', "''");
        let cmd = format!(
            "Compress-Archive -Path '{}\\*' -DestinationPath '{}' -Force",
            ps_escape(src_dir),
            ps_escape(zip_path)
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &cmd])
            .status()
            .map_err(Error::Io)?
    } else {
        Command::new("zip")
            .arg("-r")
            .arg(zip_path)
            .arg(".")
            .current_dir(src_dir)
            .status()
            .map_err(Error::Io)?
    };
    if !status.success() {
        return Err(Error::Io(std::io::Error::other(format!(
            "archiving the diagnostic bundle exited with status {status}"
        ))));
    }
    Ok(())
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
        perms::ensure_dir_0700(dst)?;
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
        perms::set_file_0600(dst)?;
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
        perms::ensure_dir_0700(&diag_dir).unwrap();

        let logs = tmp.path().join("logs");
        fs::create_dir_all(&logs).unwrap();
        write_log(&logs.join("core.log"), &["ok"]);

        let path = build_diagnostic_bundle(&diag_dir, "0190abcd-launch", &[&logs]).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "bundle file must be 0o600, got {mode:o}");
        }
        assert!(path.starts_with(&diag_dir));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("mneme-diagnostic-"));
        assert!(name.ends_with(".zip"));
        assert!(name.contains("0190abcd-launch"));
    }

    /// Read every member of a zip archive's content as one concatenated string
    /// — `unzip -p` on macOS/Linux, `Expand-Archive` + read-back on Windows
    /// (which has no bundled `unzip.exe`).
    fn unzip_all_text(bundle: &Path) -> String {
        if cfg!(target_os = "windows") {
            let out_dir = tempdir().unwrap();
            let status = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        bundle.display().to_string().replace('\'', "''"),
                        out_dir.path().display().to_string().replace('\'', "''"),
                    ),
                ])
                .status()
                .expect("powershell available on windows");
            assert!(status.success(), "Expand-Archive failed");
            let mut combined = String::new();
            fn walk(dir: &Path, out: &mut String) {
                for entry in fs::read_dir(dir).unwrap().flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        walk(&p, out);
                    } else if let Ok(s) = fs::read_to_string(&p) {
                        out.push_str(&s);
                    }
                }
            }
            walk(out_dir.path(), &mut combined);
            combined
        } else {
            let unzipped = Command::new("unzip")
                .arg("-p")
                .arg(bundle)
                .output()
                .expect("unzip available on macos/linux");
            assert!(unzipped.status.success(), "unzip -p failed");
            String::from_utf8_lossy(&unzipped.stdout).into_owned()
        }
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
        let body = unzip_all_text(&bundle);

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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&diag_dir).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700);
        }
        assert!(diag_dir.exists());
        perms::ensure_dir_0700(&diag_dir).unwrap();
    }
}
