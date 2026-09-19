//! Structured core log + a dependency-free secret-scrubber for the diagnostic
//! bundle.
//!
//! Hand-derived from the slice-6 `logging` keeper. The scrubber (`scrub`) is the
//! security-critical part — it is ported verbatim (every redaction rule + every
//! test) because the diagnostic bundle (the `menu` keeper) ships log files to a
//! user's support contact, and a missed secret there is a real leak. The only
//! change from slice 6 is cosmetic: launch/warn lines go through `log::` instead
//! of `eprintln!`, matching the rest of the slice-7 crate.
//!
//! macOS / unix only (`~/Library/Logs`, `/bin/date`); slice-8 cross-platform.

//! macOS / Linux / Windows. Log directory and today's date are resolved
//! per-OS below (see [`dirs_logs_dir`] and [`today_yyyymmdd`]) rather than
//! hardcoding `~/Library/Logs` and shelling out to `/bin/date`, neither of
//! which exists on Windows.

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use regex::Regex;

use uuid::Uuid;

use crate::errors::Error;

pub struct LogContext {
    pub launch_uuid: String,
    // The live append handle to today's core log. Held in Tauri state for the
    // life of the process; the launch record is written at open time and future
    // runtime log lines append here. Not read back in-process today (the
    // diagnostic bundle reads the flushed file from disk), so it is allow'd —
    // same write-only / tracked-cleanup convention used elsewhere.
    #[allow(dead_code)]
    pub core_log: Mutex<BufWriter<File>>,
}

pub fn open_core_log(home: &Path) -> Result<LogContext, Error> {
    let logs_dir = dirs_logs_dir().join("com.mneme.desktop");
    create_dir_all(&logs_dir)?;
    let date = today_yyyymmdd();
    let path = logs_dir.join(format!("core-{date}.log"));
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    // Windows: no mode-bit equivalent to set here; see perms.rs module doc for
    // why this app relies on NTFS profile-directory inheritance on Windows.
    let launch_uuid = Uuid::now_v7().to_string();
    let mut w = BufWriter::new(file);
    let _ = writeln!(
        w,
        r#"{{"event":"launch","launch_uuid":"{launch_uuid}","home":"{}"}}"#,
        home.display()
    );
    Ok(LogContext {
        launch_uuid,
        core_log: Mutex::new(w),
    })
}

/// Base directory logs live under, before the `com.mneme.desktop` subfolder.
/// macOS keeps the platform-conventional `~/Library/Logs`; other platforms
/// (Linux, Windows) use the OS's "local data" root (`~/.local/share` /
/// `%LOCALAPPDATA%`) with a `Logs` subfolder — a reasonable, portable default
/// rather than each OS's most idiomatic-possible location.
pub fn dirs_logs_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .expect("could not determine the user's home directory")
            .join("Library")
            .join("Logs")
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::data_local_dir()
            .expect("could not determine the user's local data directory")
            .join("Logs")
    }
}

/// Today's date as `YYYYMMDD`, UTC. Computed with plain integer arithmetic
/// (Howard Hinnant's `civil_from_days` algorithm) instead of shelling out to
/// `/bin/date`, which doesn't exist on Windows — this only needs a stable
/// rotation key for the log filename, not a timezone-correct local date.
fn today_yyyymmdd() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}{m:02}{d:02}")
}

/// Days-since-epoch (1970-01-01) -> (year, month, day), proleptic Gregorian,
/// UTC. http://howardhinnant.github.io/date_algorithms.html#civil_from_days
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Literal substrings that trigger token-style redaction (consume from the
/// match position to the next delimiter). Case-sensitive on purpose —
/// header-shaped redactions live in `redact_header_line` below and run
/// case-insensitively over the whole line.
const SENSITIVE_PATTERNS: &[&str] = &["session=", "password", "argon2id$", "sk_"];

/// Hard cap on the per-line scan length. `scrub` is fed by an iteration over
/// log lines that, in pathological cases, may not be newline-terminated (e.g.
/// non-UTF-8 binary blobs sneaking into a log). Anything longer than this is
/// truncated with a `[SCRUB-TRUNCATED]` marker so we never burn unbounded CPU
/// on a giant single line inside the diagnostic-bundle path.
const MAX_LINE_BYTES: usize = 64 * 1024;

/// Strengthened denylist scrubber for the diagnostic bundle.
///
/// Order matters: the case-insensitive header redactions run first so they
/// catch the full header line (including the secret value to end-of-line),
/// then the structured-token denylist runs, then the catch-all hex run.
///
/// We deliberately do not pull in the `regex` crate — each pattern is a byte
/// scan + `find`, so the scrub path stays dependency-free and audit-friendly.
pub fn scrub(line: &str) -> String {
    // 1. Length cap. Cheap defense against a hostile single-line log entry.
    let working: std::borrow::Cow<str> = if line.len() > MAX_LINE_BYTES {
        let mut s = line[..MAX_LINE_BYTES].to_string();
        s.push_str(" [SCRUB-TRUNCATED]");
        std::borrow::Cow::Owned(s)
    } else {
        std::borrow::Cow::Borrowed(line)
    };
    let mut out = working.into_owned();

    // 2. Case-insensitive header redactions. Redact the entire value to EOL so
    //    cookies / bearer tokens with internal `=`, `;`, spaces are all consumed
    //    regardless of internal punctuation.
    out = redact_header_line(&out, "authorization:");
    out = redact_header_line(&out, "cookie:");
    out = redact_header_line(&out, "set-cookie:");
    out = redact_header_line(&out, "proxy-authorization:");

    // 3. PAT-shaped tokens and URL userinfo (regex-based, compiled once).
    out = redact_pat_shapes(&out);

    // 4. Structured-secret tokens (substring -> consume to next delimiter).
    for p in SENSITIVE_PATTERNS {
        while let Some(pos) = out.find(p) {
            let rest = &out[pos..];
            let end_offset = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == '}')
                .unwrap_or(rest.len());
            out.replace_range(pos..pos + end_offset, "[SCRUBBED]");
        }
    }

    // 5. ENV-style dumps: `FOO_SECRET=...`, `BAR_TOKEN=...`, etc.
    out = redact_env_dumps(&out);

    // 6. JWT (three dotted base64url segments, requiring the standard `eyJ`
    //    header prefix that base64-encodes `{"`).
    out = redact_jwts(&out);

    // 7. Long hex runs (32+ chars) — catches SESSION_SECRET-shaped values,
    //    foreign SHA-256 hashes, any bare hex blob in a log.
    out = redact_long_hex(&out);

    out
}

fn redact_header_line(input: &str, header_lower: &str) -> String {
    let lc = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    while cursor < input.len() {
        if let Some(rel) = lc[cursor..].find(header_lower) {
            let start = cursor + rel;
            // Only redact when the header name is at a real boundary: at string
            // start OR immediately after a newline / `, ` / `; ` etc.
            let at_boundary = start == 0
                || matches!(
                    input.as_bytes()[start - 1],
                    b'\n' | b'\r' | b' ' | b'\t' | b'"' | b'\'' | b'{' | b',' | b';'
                );
            if !at_boundary {
                out.push_str(&input[cursor..start + header_lower.len()]);
                cursor = start + header_lower.len();
                continue;
            }
            let end_rel = input[start..]
                .find(['\n', '\r'])
                .map(|p| start + p)
                .unwrap_or(input.len());
            out.push_str(&input[cursor..start]);
            out.push_str(header_lower);
            out.push_str(" [SCRUBBED]");
            cursor = end_rel;
        } else {
            out.push_str(&input[cursor..]);
            break;
        }
    }
    out
}

fn redact_env_dumps(input: &str) -> String {
    // Match `[A-Z_]+_(SECRET|TOKEN|KEY|PASSWORD)=<non-space>+`.
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_uppercase() || b == b'_' {
            let tok_start = i;
            let mut j = i;
            while j < bytes.len()
                && (bytes[j].is_ascii_uppercase() || bytes[j].is_ascii_digit() || bytes[j] == b'_')
            {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                let token = &input[tok_start..j];
                let upper_suffix_match = ["_SECRET", "_TOKEN", "_KEY", "_PASSWORD"]
                    .iter()
                    .any(|s| token.ends_with(s));
                if upper_suffix_match {
                    let mut k = j + 1;
                    while k < bytes.len()
                        && !bytes[k].is_ascii_whitespace()
                        && bytes[k] != b'"'
                        && bytes[k] != b','
                        && bytes[k] != b'}'
                    {
                        k += 1;
                    }
                    out.push_str(token);
                    out.push_str("=[SCRUBBED]");
                    i = k;
                    continue;
                }
            }
            out.push_str(&input[tok_start..j]);
            i = j;
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn redact_jwts(input: &str) -> String {
    // Conservative JWT: starts with `eyJ`, two dots separating three base64url
    // segments. Anything matching the shape is treated as a JWT and scrubbed.
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"eyJ" {
            let mut j = i + 3;
            let is_b64 = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
            while j < bytes.len() && is_b64(bytes[j]) {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'.' && j > i + 3 {
                let mut k = j + 1;
                while k < bytes.len() && is_b64(bytes[k]) {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b'.' && k > j + 1 {
                    let mut m = k + 1;
                    while m < bytes.len() && is_b64(bytes[m]) {
                        m += 1;
                    }
                    if m > k + 1 {
                        out.push_str("[SCRUBBED-JWT]");
                        i = m;
                        continue;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Compiled regex patterns for PAT shapes that must never appear in the
/// diagnostic bundle. Compiled once at first use via `OnceLock`.
static PAT_REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();

fn pat_regexes() -> &'static Vec<Regex> {
    PAT_REGEXES.get_or_init(|| {
        vec![
            // URL userinfo — strip the whole `user:token@` or `token@` segment so
            // the host is preserved but credentials are not. Run this FIRST so the
            // bare token patterns below don't need to see inside URLs.
            Regex::new(r"://[^/@\s]+@").unwrap(),
            // GitHub classic tokens: ghp_, gho_, ghu_, ghs_, ghr_
            Regex::new(r"(?i)\b(gh[pousr])_[A-Za-z0-9]{16,}\b").unwrap(),
            // GitHub fine-grained PATs (real ones have 70+ char suffixes; 10 is a
            // conservative floor that still covers test tokens and avoids false positives
            // on short identifiers)
            Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{10,}\b").unwrap(),
            // GitLab PATs (real ones are 20+ chars; 10 avoids false positives while
            // covering test tokens)
            Regex::new(r"\bglpat-[A-Za-z0-9_-]{10,}\b").unwrap(),
        ]
    })
}

/// Redact PAT-shaped tokens and URL userinfo from `input`.
///
/// The URL userinfo rule (`://user:token@host`) is applied first so that a
/// PAT embedded as a URL password is caught even if the bare-token patterns
/// would otherwise miss it in context.
fn redact_pat_shapes(input: &str) -> String {
    let mut out = input.to_owned();
    for re in pat_regexes().iter() {
        out = re
            .replace_all(&out, |caps: &regex::Captures| {
                let m = caps.get(0).unwrap().as_str();
                // For URL userinfo (`://stuff@`), preserve the scheme separator
                // and the `@` so the host portion stays readable.
                if m.starts_with("://") && m.ends_with('@') {
                    "://<redacted>@".to_owned()
                } else {
                    "[SCRUBBED]".to_owned()
                }
            })
            .into_owned();
    }
    out
}

fn redact_long_hex(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_hexdigit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            let run = i - start;
            // Boundary check: only treat as a hex blob if it isn't sandwiched
            // inside a larger alphanumeric token.
            let prev_ok = start == 0
                || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
            let next_ok =
                i == bytes.len() || !(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_');
            if run >= 32 && prev_ok && next_ok {
                out.push_str("[SCRUBBED-HEX]");
            } else {
                out.push_str(&input[start..i]);
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

pub fn rotate_old_logs(logs_dir: &Path, keep_days: u32) -> std::io::Result<()> {
    use std::time::{Duration, SystemTime};
    let cutoff = SystemTime::now() - Duration::from_secs(60 * 60 * 24 * keep_days as u64);
    for entry in std::fs::read_dir(logs_dir)?.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime < cutoff {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_replaces_session_token() {
        let scrubbed = scrub(r#"foo session=abc123 bar"#);
        assert!(!scrubbed.contains("abc123"));
        assert!(scrubbed.contains("[SCRUBBED]"));
    }

    #[test]
    fn scrub_handles_password() {
        let scrubbed = scrub("password=hunter2");
        assert!(!scrubbed.contains("hunter2"));
    }

    #[test]
    fn scrub_idempotent_on_clean_line() {
        let line = "regular log line with nothing sensitive";
        assert_eq!(scrub(line), line);
    }

    #[test]
    fn scrub_redacts_authorization_header() {
        let line = r#"{"level":30,"msg":"req","Authorization: Bearer abc123secrettoken"}"#;
        let scrubbed = scrub(line);
        assert!(
            !scrubbed.contains("abc123secrettoken"),
            "bearer must be gone"
        );
        assert!(scrubbed.to_lowercase().contains("authorization:"));
    }

    #[test]
    fn scrub_redacts_cookie_header() {
        let scrubbed = scrub("Cookie: session=verysecretvalue\nnext line");
        assert!(!scrubbed.contains("verysecretvalue"));
        assert!(scrubbed.contains("next line"));
    }

    #[test]
    fn scrub_redacts_set_cookie_header() {
        let scrubbed = scrub("Set-Cookie: id=abcdefverysecret; Path=/");
        assert!(!scrubbed.contains("abcdefverysecret"));
    }

    #[test]
    fn scrub_redacts_session_secret_shaped_hex() {
        let line = "loaded SESSION_SECRET=4f6b3c1a9d8e2b5c7f0a1b2c3d4e5f60718293a4b5c6d7e8f0a1b2c3d4e5f6071 from keychain";
        let scrubbed = scrub(line);
        assert!(
            !scrubbed.contains("4f6b3c1a9d8e2b5c7f0a1b2c3d4e5f60718293a4b5c6d7e8f0a1b2c3d4e5f6071"),
            "session secret hex must be scrubbed"
        );
    }

    #[test]
    fn scrub_redacts_env_dump_patterns() {
        let scrubbed = scrub("env: API_TOKEN=xyzsecretvalue OTHER=ok DB_PASSWORD=pwd1234secret");
        assert!(!scrubbed.contains("xyzsecretvalue"));
        assert!(!scrubbed.contains("pwd1234secret"));
        assert!(scrubbed.contains("OTHER=ok"));
    }

    #[test]
    fn scrub_redacts_jwt() {
        let line = "auth jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.abc123-_ABCdef trailing";
        let scrubbed = scrub(line);
        assert!(!scrubbed.contains("eyJhbGciOiJIUzI1NiJ9"));
        assert!(scrubbed.contains("trailing"));
    }

    #[test]
    fn scrub_handles_giant_line_with_truncation_marker() {
        let mut huge = String::with_capacity(MAX_LINE_BYTES + 1024);
        for _ in 0..(MAX_LINE_BYTES / 4) {
            huge.push_str("abcd");
        }
        huge.push_str(" tail-not-scanned-because-truncated");
        let scrubbed = scrub(&huge);
        assert!(scrubbed.contains("[SCRUB-TRUNCATED]"));
        assert!(!scrubbed.contains("tail-not-scanned-because-truncated"));
    }

    #[test]
    fn scrub_leaves_short_hex_alone() {
        let line = "version sha 1abc def 0123456789ab"; // 12 hex chars — under threshold
        assert_eq!(scrub(line), line);
    }

    #[test]
    fn scrub_redacts_pat_shapes_and_url_userinfo() {
        for s in [
            "ghp_abcDEF1234567890xyz",
            "github_pat_11ABCDEFG_xyz123456",
            "glpat-abc123XYZ456def",
        ] {
            assert!(
                !scrub(&format!("pushing with token {s}")).contains(s),
                "{s} leaked"
            );
        }
        let u = "https://x-access-token:ghp_secrettoken123456@github.com/me/n.git";
        let out = scrub(&format!("remote = {u}"));
        assert!(
            !out.contains("ghp_secrettoken123456"),
            "token in URL leaked"
        );
        assert!(
            !out.contains(":ghp_secrettoken123456@"),
            "userinfo not stripped"
        );
    }

    #[test]
    fn rotate_old_logs_removes_old() {
        use tempfile::tempdir;
        let tmp = tempdir().unwrap();
        let old = tmp.path().join("old.log");
        let new = tmp.path().join("new.log");
        std::fs::write(&old, b"").unwrap();
        std::fs::write(&new, b"").unwrap();
        // Set old mtime to 10 days ago.
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(60 * 60 * 24 * 10);
        let ft = filetime::FileTime::from_system_time(when);
        filetime::set_file_mtime(&old, ft).unwrap();
        rotate_old_logs(tmp.path(), 7).unwrap();
        assert!(!old.exists(), "old log should be removed");
        assert!(new.exists(), "new log should remain");
    }
}
