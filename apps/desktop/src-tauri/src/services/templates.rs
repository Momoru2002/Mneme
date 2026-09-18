//! Template operations for the Mneme desktop app. Templates are Markdown files
//! stored under `~/.mneme/templates/` (filesystem-backed, no DB).
//!
//! Ports `apps/api/src/services/template-service.ts` 1:1, adapting Node/fs
//! async idioms to Rust + std::fs. Timestamps returned as Unix-ms (i64) to
//! match the slice-7 convention (Phase-5 aligns the TS side).
//!
//! Path-safety note: templates live inside `templates_dir` (an absolute path
//! computed from `~/.mneme/templates`). The ONLY path-safety requirement here
//! is that a caller-supplied template name cannot escape the templates dir.
//! We enforce this the same way the TS service does: `path.basename(name) ==
//! name` (i.e. the name contains no path separators), plus the name-validation
//! rules from `templateNameSchema`. No `resolve_well_path` is needed because
//! templates are NOT inside a Well.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::dto::{ApplyTemplateResult, TemplateDto, TemplateListEntry};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum TemplatesError {
    #[error("template not found")]
    NotFound,
    #[error("template already exists")]
    AlreadyExists,
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

impl TemplatesError {
    pub fn client_message(&self) -> String {
        match self {
            TemplatesError::NotFound => "template not found".into(),
            TemplatesError::AlreadyExists => "template already exists".into(),
            TemplatesError::InvalidName(m) => format!("invalid name: {m}"),
            TemplatesError::Io(_) => "internal error".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Template name validation (ports templateNameSchema from shared/schemas/templates.ts)
// ---------------------------------------------------------------------------

const ILLEGAL_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

fn validate_name(name: &str) -> Result<(), TemplatesError> {
    if name.is_empty() {
        return Err(TemplatesError::InvalidName("name is required".into()));
    }
    // Guard null bytes: on Linux, open(2) truncates C strings at \0, so two
    // names differing only after a null byte would silently collide.
    if name.contains('\0') {
        return Err(TemplatesError::InvalidName(
            "name must not contain null bytes".into(),
        ));
    }
    if name.chars().count() > 128 {
        return Err(TemplatesError::InvalidName(
            "name must be at most 128 characters".into(),
        ));
    }
    if name.chars().any(|c| ILLEGAL_CHARS.contains(&c)) {
        return Err(TemplatesError::InvalidName(
            "name contains illegal characters".into(),
        ));
    }
    if name.starts_with('.') {
        return Err(TemplatesError::InvalidName(
            "name must not start with a dot".into(),
        ));
    }
    if name != name.trim() {
        return Err(TemplatesError::InvalidName(
            "name must not start or end with whitespace".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

const TEMPLATE_EXT: &str = ".md";

/// Resolve the absolute path for a template file inside `templates_dir`.
/// Validates the name and ensures it cannot escape the directory, including a
/// symlink-escape check that mirrors `resolve_well_path`'s final guard.
fn path_for(templates_dir: &Path, name: &str) -> Result<PathBuf, TemplatesError> {
    validate_name(name)?;
    // Mirror TS `path.basename(name) === name`: reject if the name contains
    // any path separator (forward or back slash — ILLEGAL_CHARS already covers
    // them, but an explicit `.parent()` check is belt-and-suspenders).
    let with_ext = if name.ends_with(TEMPLATE_EXT) {
        name.to_string()
    } else {
        format!("{name}{TEMPLATE_EXT}")
    };
    // Component check: the filename must not navigate up/sideways.
    let p = Path::new(&with_ext);
    if p.components().count() != 1 {
        return Err(TemplatesError::InvalidName(
            "name must not contain path separators".into(),
        ));
    }
    let abs = templates_dir.join(&with_ext);
    // Symlink-escape check: if the path already exists and it is a symlink,
    // verify its real location is still inside the templates directory.
    if abs.exists() {
        let real = std::fs::canonicalize(&abs).map_err(TemplatesError::Io)?;
        let dir_real =
            std::fs::canonicalize(templates_dir).unwrap_or_else(|_| templates_dir.to_path_buf());
        if real != dir_real && !real.starts_with(&dir_real) {
            return Err(TemplatesError::InvalidName(
                "name resolves to a path outside the templates directory".into(),
            ));
        }
    }
    Ok(abs)
}

fn strip_ext(filename: &str) -> &str {
    filename.strip_suffix(TEMPLATE_EXT).unwrap_or(filename)
}

fn mtime_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn file_size(path: &Path) -> i64 {
    std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0)
}

/// Ensure the templates directory exists, creating it if needed.
fn ensure_templates_dir(dir: &Path) -> Result<(), std::io::Error> {
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API (pure functions, testable without Tauri runtime)
// ---------------------------------------------------------------------------

/// List all templates in the directory, sorted by name (ports `listTemplates`).
pub fn list(templates_dir: &Path) -> Result<Vec<TemplateListEntry>, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let rd = match std::fs::read_dir(templates_dir) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(TemplatesError::Io(e)),
    };
    let mut out: Vec<TemplateListEntry> = rd
        .flatten()
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.file_name().to_string_lossy().ends_with(TEMPLATE_EXT)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .map(|e| {
            let name_os = e.file_name();
            let name_str = name_os.to_string_lossy();
            let name = strip_ext(&name_str).to_string();
            let full = e.path();
            TemplateListEntry {
                name,
                size: file_size(&full),
                mtime: mtime_ms(&full),
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Read a single template by name (ports `readTemplate`).
pub fn get(templates_dir: &Path, name: &str) -> Result<TemplateDto, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let abs = path_for(templates_dir, name)?;
    // Avoid TOCTOU: don't check exists() then open separately. Instead read
    // directly and map NotFound. Also use content.len() for byte count and a
    // single metadata() call for mtime — avoids extra stat(2) round-trips.
    let content = std::fs::read_to_string(&abs).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TemplatesError::NotFound
        } else {
            TemplatesError::Io(e)
        }
    })?;
    let meta = std::fs::metadata(&abs).ok();
    let size = content.len() as i64;
    let mtime = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let display_name = strip_ext(
        abs.file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
            .as_ref(),
    )
    .to_string();
    Ok(TemplateDto {
        path: abs.to_string_lossy().into_owned(),
        name: display_name,
        size,
        mtime,
        content,
    })
}

/// Create a new template (ports `createTemplate`). Errors if name already exists.
pub fn create(
    templates_dir: &Path,
    name: &str,
    content: &str,
) -> Result<TemplateDto, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let abs = path_for(templates_dir, name)?;
    // Avoid TOCTOU: use OpenOptions with create_new=true which atomically
    // rejects the write if the file already exists.
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&abs)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                TemplatesError::AlreadyExists
            } else {
                TemplatesError::Io(e)
            }
        })?
        .write_all(content.as_bytes())
        .map_err(TemplatesError::Io)?;
    get(templates_dir, name)
}

/// Update an existing template's content (ports `updateTemplate`). Errors if not found.
pub fn update(
    templates_dir: &Path,
    name: &str,
    content: &str,
) -> Result<TemplateDto, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let abs = path_for(templates_dir, name)?;
    // Avoid TOCTOU: open with truncate but NOT create, so the open fails with
    // NotFound if the file does not exist rather than creating it silently.
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(&abs)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TemplatesError::NotFound
            } else {
                TemplatesError::Io(e)
            }
        })?
        .write_all(content.as_bytes())
        .map_err(TemplatesError::Io)?;
    get(templates_dir, name)
}

/// Delete a template (ports `deleteTemplate`). Errors if not found.
pub fn remove(templates_dir: &Path, name: &str) -> Result<(), TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let abs = path_for(templates_dir, name)?;
    // Avoid TOCTOU: remove directly and map NotFound from the error kind.
    std::fs::remove_file(&abs).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TemplatesError::NotFound
        } else {
            TemplatesError::Io(e)
        }
    })
}

/// Duplicate a template, appending "(copy)" / "(copy N)" suffixes as needed
/// (ports `duplicateTemplate`).
pub fn duplicate(templates_dir: &Path, name: &str) -> Result<TemplateDto, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let src_abs = path_for(templates_dir, name)?;
    // Read the source content directly — if it doesn't exist, read_to_string
    // gives NotFound, avoiding a TOCTOU exists()-then-read race.
    let src_content = std::fs::read_to_string(&src_abs).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            TemplatesError::NotFound
        } else {
            TemplatesError::Io(e)
        }
    })?;
    // Find an available copy name. Guard against runaway loops (the TS source
    // has the same bug; we fix it here). 1 000 copies is a reasonable ceiling.
    const MAX_SUFFIX: u32 = 1_000;
    let base = strip_ext(name).to_string();
    let candidate_name = format!("{base} (copy)");
    let dest_name = if !path_for(templates_dir, &candidate_name)?.exists() {
        candidate_name
    } else {
        let mut suffix = 2u32;
        loop {
            if suffix > MAX_SUFFIX {
                return Err(TemplatesError::AlreadyExists);
            }
            let c = format!("{base} (copy {suffix})");
            if !path_for(templates_dir, &c)?.exists() {
                break c;
            }
            suffix += 1;
        }
    };
    let dest_abs = path_for(templates_dir, &dest_name)?;
    // Write using create_new so a concurrent creator can't clobber us.
    use std::io::Write as _;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest_abs)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                TemplatesError::AlreadyExists
            } else {
                TemplatesError::Io(e)
            }
        })?
        .write_all(src_content.as_bytes())
        .map_err(TemplatesError::Io)?;
    get(templates_dir, &dest_name)
}

// ---------------------------------------------------------------------------
// Variable interpolation (ports `applyVars` + `renderTemplate`)
// ---------------------------------------------------------------------------

/// Return true if `key` is a valid placeholder identifier — mirrors the TS
/// regex `/\{\{\s*([a-zA-Z0-9_.\-]+)\s*\}\}/g`. Keys containing spaces or
/// other chars that don't match this pattern are left intact in the output
/// (identical behaviour to the TS reference implementation).
fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Render a template by substituting `{{var}}` placeholders. Built-in vars:
/// `date` (YYYY-MM-DD) and `datetime` (ISO-8601). `{{cursor}}` is preserved.
/// Ports `applyVars` in template-service.ts.
///
/// Only keys matching `[a-zA-Z0-9_.-]+` (same as the TS regex) are looked up;
/// placeholders with spaces or other characters are left entirely intact.
/// Uses a manual scan rather than an external regex crate to avoid adding a
/// dependency (the project's Cargo.toml does not include `regex`).
pub fn apply_vars(source: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let now = chrono_lite_now();
    let date = &now.0;
    let datetime = &now.1;
    let mut result = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find("{{") {
        let before = &rest[..open];
        result.push_str(before);
        rest = &rest[open + 2..];
        if let Some(close) = rest.find("}}") {
            let inner = &rest[..close];
            let key = inner.trim();
            rest = &rest[close + 2..];
            if !is_valid_key(key) {
                // Key contains spaces or invalid chars — preserve the original
                // placeholder exactly as written (mirrors TS behaviour).
                result.push_str("{{");
                result.push_str(inner);
                result.push_str("}}");
            } else if key == "cursor" {
                result.push_str("{{cursor}}");
            } else if key == "date" {
                result.push_str(date);
            } else if key == "datetime" {
                result.push_str(datetime);
            } else if let Some(val) = vars.get(key) {
                result.push_str(val);
            } else {
                // Valid key but no value supplied — preserve the placeholder.
                result.push_str("{{");
                result.push_str(key);
                result.push_str("}}");
            }
        } else {
            // No closing `}}` — output the `{{` and continue
            result.push_str("{{");
        }
    }
    result.push_str(rest);
    result
}

/// Apply a template by name, substituting vars (ports `renderTemplate`).
pub fn apply(
    templates_dir: &Path,
    name: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Result<ApplyTemplateResult, TemplatesError> {
    let tmpl = get(templates_dir, name)?;
    Ok(ApplyTemplateResult {
        rendered: apply_vars(&tmpl.content, vars),
    })
}

// ---------------------------------------------------------------------------
// Import defaults (ports `importDefaults`)
// ---------------------------------------------------------------------------

struct DefaultTemplate {
    name: &'static str,
    content: &'static str,
}

const DEFAULTS: &[DefaultTemplate] = &[
    DefaultTemplate {
        name: "Coursera Lecture",
        content: "---\ntype: coursera-lecture\ncourse: \"{{course}}\"\nweek: \"{{week}}\"\ntitle: \"{{title}}\"\ndate: {{date}}\ntags: [coursera, lecture]\n---\n\n# {{title}}\n\n## Key concepts\n\n{{cursor}}\n\n## Notes\n\n## Questions to revisit\n\n## References\n",
    },
    DefaultTemplate {
        name: "Coursera Reading",
        content: "---\ntype: coursera-reading\ncourse: \"{{course}}\"\nweek: \"{{week}}\"\ntitle: \"{{title}}\"\ndate: {{date}}\ntags: [coursera, reading]\n---\n\n# {{title}}\n\n## Summary\n\n{{cursor}}\n\n## Highlights\n\n## Takeaways\n",
    },
    DefaultTemplate {
        name: "Coursera Quiz",
        content: "---\ntype: coursera-quiz\ncourse: \"{{course}}\"\nweek: \"{{week}}\"\ntitle: \"{{title}}\"\ndate: {{date}}\ntags: [coursera, quiz]\nscore: \"\"\n---\n\n# {{title}}\n\n## Attempt log\n\n{{cursor}}\n\n## Wrong answers (review)\n\n## Lessons\n",
    },
    DefaultTemplate {
        name: "Coursera Lab",
        content: "---\ntype: coursera-lab\ncourse: \"{{course}}\"\nweek: \"{{week}}\"\ntitle: \"{{title}}\"\ndate: {{date}}\ntags: [coursera, lab]\n---\n\n# {{title}}\n\n## Objective\n\n{{cursor}}\n\n## Steps\n\n## Results\n\n## Reflection\n",
    },
];

/// Import the built-in default templates, skipping any that already exist.
/// Returns the list of newly created entries (ports `importDefaults`).
pub fn import_defaults(templates_dir: &Path) -> Result<Vec<TemplateListEntry>, TemplatesError> {
    ensure_templates_dir(templates_dir)?;
    let mut created = Vec::new();
    for t in DEFAULTS {
        let abs = path_for(templates_dir, t.name)?;
        // Use create_new=true to atomically skip existing files (TOCTOU-safe).
        use std::io::Write as _;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&abs)
        {
            Ok(mut f) => {
                f.write_all(t.content.as_bytes())
                    .map_err(TemplatesError::Io)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Template already exists — skip it.
                continue;
            }
            Err(e) => return Err(TemplatesError::Io(e)),
        }
        let tmpl = get(templates_dir, t.name)?;
        created.push(TemplateListEntry {
            name: tmpl.name,
            size: tmpl.size,
            mtime: tmpl.mtime,
        });
    }
    Ok(created)
}

// ---------------------------------------------------------------------------
// Tiny helpers (no extra crate deps)
// ---------------------------------------------------------------------------

/// Return (date YYYY-MM-DD, datetime ISO-8601) for the current wall-clock time.
fn chrono_lite_now() -> (String, String) {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Calendar math (no chrono dep — the Cargo.toml doesn't include it)
    let days_since_epoch = secs / 86400;
    // Gregorian calendar calculation
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);
    let secs_of_day = secs % 86400;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    let date = format!("{year:04}-{month:02}-{day:02}");
    let datetime = format!("{date}T{hh:02}:{mm:02}:{ss:02}Z");
    (date, datetime)
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
/// Uses the proleptic Gregorian calendar algorithm.
fn days_to_ymd(z: i64) -> (i64, i64, i64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn tdir(d: &TempDir) -> &Path {
        d.path()
    }

    // --- list ---

    #[test]
    fn list_empty_dir_returns_empty() {
        let d = tmp();
        let result = list(tdir(&d)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_returns_md_files_sorted() {
        let d = tmp();
        std::fs::write(d.path().join("beta.md"), "b").unwrap();
        std::fs::write(d.path().join("alpha.md"), "a").unwrap();
        std::fs::write(d.path().join("not_md.txt"), "x").unwrap();
        std::fs::write(d.path().join(".hidden.md"), "h").unwrap();

        let result = list(tdir(&d)).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "beta");
    }

    // --- get ---

    #[test]
    fn get_returns_content_and_metadata() {
        let d = tmp();
        std::fs::write(d.path().join("My Note.md"), "hello world").unwrap();
        let tmpl = get(tdir(&d), "My Note").unwrap();
        assert_eq!(tmpl.name, "My Note");
        assert_eq!(tmpl.content, "hello world");
        assert_eq!(tmpl.size, 11);
        assert!(tmpl.mtime > 0);
    }

    #[test]
    fn get_not_found_returns_error() {
        let d = tmp();
        assert!(matches!(
            get(tdir(&d), "ghost"),
            Err(TemplatesError::NotFound)
        ));
    }

    // --- create ---

    #[test]
    fn create_writes_file_and_returns_dto() {
        let d = tmp();
        let tmpl = create(tdir(&d), "NewNote", "content here").unwrap();
        assert_eq!(tmpl.name, "NewNote");
        assert_eq!(tmpl.content, "content here");
        assert!(d.path().join("NewNote.md").exists());
    }

    #[test]
    fn create_rejects_duplicate() {
        let d = tmp();
        create(tdir(&d), "Dup", "a").unwrap();
        assert!(matches!(
            create(tdir(&d), "Dup", "b"),
            Err(TemplatesError::AlreadyExists)
        ));
    }

    // --- update ---

    #[test]
    fn update_overwrites_content() {
        let d = tmp();
        create(tdir(&d), "Note", "old").unwrap();
        let tmpl = update(tdir(&d), "Note", "new content").unwrap();
        assert_eq!(tmpl.content, "new content");
    }

    #[test]
    fn update_not_found_returns_error() {
        let d = tmp();
        assert!(matches!(
            update(tdir(&d), "ghost", "x"),
            Err(TemplatesError::NotFound)
        ));
    }

    // --- remove ---

    #[test]
    fn remove_deletes_file() {
        let d = tmp();
        create(tdir(&d), "ToDelete", "x").unwrap();
        remove(tdir(&d), "ToDelete").unwrap();
        assert!(!d.path().join("ToDelete.md").exists());
    }

    #[test]
    fn remove_not_found_returns_error() {
        let d = tmp();
        assert!(matches!(
            remove(tdir(&d), "ghost"),
            Err(TemplatesError::NotFound)
        ));
    }

    // --- duplicate ---

    #[test]
    fn duplicate_creates_copy_suffix() {
        let d = tmp();
        create(tdir(&d), "Original", "content").unwrap();
        let copy = duplicate(tdir(&d), "Original").unwrap();
        assert_eq!(copy.name, "Original (copy)");
        assert_eq!(copy.content, "content");
        assert!(d.path().join("Original (copy).md").exists());
    }

    #[test]
    fn duplicate_increments_suffix_when_copy_exists() {
        let d = tmp();
        create(tdir(&d), "Note", "c").unwrap();
        create(tdir(&d), "Note (copy)", "c").unwrap();
        let copy2 = duplicate(tdir(&d), "Note").unwrap();
        assert_eq!(copy2.name, "Note (copy 2)");
    }

    #[test]
    fn duplicate_not_found_returns_error() {
        let d = tmp();
        assert!(matches!(
            duplicate(tdir(&d), "ghost"),
            Err(TemplatesError::NotFound)
        ));
    }

    // --- apply_vars ---

    #[test]
    fn apply_vars_substitutes_custom_vars() {
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "My Title".to_string());
        let rendered = apply_vars("# {{title}}", &vars);
        assert_eq!(rendered, "# My Title");
    }

    #[test]
    fn apply_vars_preserves_cursor() {
        let vars = HashMap::new();
        let rendered = apply_vars("start {{cursor}} end", &vars);
        assert_eq!(rendered, "start {{cursor}} end");
    }

    #[test]
    fn apply_vars_substitutes_date_and_datetime() {
        let vars = HashMap::new();
        let rendered = apply_vars("{{date}} {{datetime}}", &vars);
        // Split into the two space-separated tokens.
        let parts: Vec<&str> = rendered.splitn(2, ' ').collect();
        assert_eq!(
            parts.len(),
            2,
            "expected 'date datetime' separated by space"
        );
        let date_part = parts[0];
        let datetime_part = parts[1];
        // date must match YYYY-MM-DD (10 chars, hyphens at positions 4 and 7)
        assert_eq!(date_part.len(), 10, "date should be YYYY-MM-DD (10 chars)");
        assert_eq!(&date_part[4..5], "-", "date hyphen at position 4");
        assert_eq!(&date_part[7..8], "-", "date hyphen at position 7");
        assert!(
            date_part[..4].chars().all(|c| c.is_ascii_digit()),
            "year digits"
        );
        assert!(
            date_part[5..7].chars().all(|c| c.is_ascii_digit()),
            "month digits"
        );
        assert!(
            date_part[8..10].chars().all(|c| c.is_ascii_digit()),
            "day digits"
        );
        // datetime must match YYYY-MM-DDTHH:MM:SSZ
        assert!(
            datetime_part.len() == 20
                && &datetime_part[10..11] == "T"
                && datetime_part.ends_with('Z'),
            "datetime should be YYYY-MM-DDTHH:MM:SSZ, got: {datetime_part}"
        );
        assert_eq!(&datetime_part[13..14], ":", "datetime colon after hour");
        assert_eq!(&datetime_part[16..17], ":", "datetime colon after minute");
    }

    #[test]
    fn apply_vars_preserves_placeholder_with_invalid_key_chars() {
        let vars = HashMap::new();
        // A key with a space — TS regex leaves the full placeholder intact.
        let rendered = apply_vars("{{ hello world }}", &vars);
        assert_eq!(rendered, "{{ hello world }}");
    }

    #[test]
    fn apply_vars_preserves_unknown_placeholders() {
        let vars = HashMap::new();
        let rendered = apply_vars("{{unknown_key}}", &vars);
        assert_eq!(rendered, "{{unknown_key}}");
    }

    // --- apply ---

    #[test]
    fn apply_renders_template_with_vars() {
        let d = tmp();
        create(tdir(&d), "Tmpl", "Hello {{name}}!").unwrap();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        let result = apply(tdir(&d), "Tmpl", &vars).unwrap();
        assert_eq!(result.rendered, "Hello World!");
    }

    #[test]
    fn apply_not_found_returns_error() {
        let d = tmp();
        assert!(matches!(
            apply(tdir(&d), "ghost", &HashMap::new()),
            Err(TemplatesError::NotFound)
        ));
    }

    // --- import_defaults ---

    #[test]
    fn import_defaults_creates_four_templates() {
        let d = tmp();
        let created = import_defaults(tdir(&d)).unwrap();
        assert_eq!(created.len(), 4);
        let names: Vec<_> = created.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"Coursera Lecture"));
        assert!(names.contains(&"Coursera Reading"));
        assert!(names.contains(&"Coursera Quiz"));
        assert!(names.contains(&"Coursera Lab"));
    }

    #[test]
    fn import_defaults_skips_existing() {
        let d = tmp();
        create(tdir(&d), "Coursera Lecture", "custom").unwrap();
        let created = import_defaults(tdir(&d)).unwrap();
        // Only 3 new ones; Coursera Lecture already existed
        assert_eq!(created.len(), 3);
        // Existing file is untouched
        let existing = get(tdir(&d), "Coursera Lecture").unwrap();
        assert_eq!(existing.content, "custom");
    }

    // --- validate_name ---

    #[test]
    fn validate_name_rejects_empty() {
        assert!(matches!(
            validate_name(""),
            Err(TemplatesError::InvalidName(_))
        ));
    }

    #[test]
    fn validate_name_rejects_illegal_chars() {
        for c in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
            let name = format!("test{c}name");
            assert!(
                matches!(validate_name(&name), Err(TemplatesError::InvalidName(_))),
                "expected rejection for char: {c:?}"
            );
        }
    }

    #[test]
    fn validate_name_rejects_dot_prefix() {
        assert!(matches!(
            validate_name(".hidden"),
            Err(TemplatesError::InvalidName(_))
        ));
    }

    #[test]
    fn validate_name_rejects_whitespace_trim() {
        assert!(matches!(
            validate_name(" leading"),
            Err(TemplatesError::InvalidName(_))
        ));
        assert!(matches!(
            validate_name("trailing "),
            Err(TemplatesError::InvalidName(_))
        ));
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let name = "a".repeat(129);
        assert!(matches!(
            validate_name(&name),
            Err(TemplatesError::InvalidName(_))
        ));
    }

    // --- validate_name null byte ---

    #[test]
    fn validate_name_rejects_null_byte() {
        assert!(
            matches!(
                validate_name("foo\0bar"),
                Err(TemplatesError::InvalidName(_))
            ),
            "null byte in name should be rejected"
        );
        assert!(
            matches!(validate_name("\0"), Err(TemplatesError::InvalidName(_))),
            "lone null byte should be rejected"
        );
    }

    // --- path_for safety ---

    #[test]
    fn path_for_rejects_path_traversal_in_name() {
        let d = tmp();
        // Names with illegal chars (/ etc.) are caught by validate_name
        assert!(matches!(
            path_for(tdir(&d), "../escape"),
            Err(TemplatesError::InvalidName(_))
        ));
    }

    // --- days_to_ymd calendar algorithm ---

    #[test]
    fn days_to_ymd_unix_epoch() {
        // Day 0 is 1970-01-01
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2024-02-29 is a leap day.
        // Unix timestamp 1709164800 = 2024-02-29T00:00:00Z
        // days = 1709164800 / 86400 = 19782
        let days = 1709164800i64 / 86400;
        assert_eq!(days_to_ymd(days), (2024, 2, 29));
    }

    #[test]
    fn days_to_ymd_month_boundary() {
        // 2023-03-01 (day after end of February in a non-leap year)
        // Unix timestamp 1677628800 = 2023-03-01T00:00:00Z
        // days = 1677628800 / 86400 = 19417
        let days = 1677628800i64 / 86400;
        assert_eq!(days_to_ymd(days), (2023, 3, 1));
    }

    #[test]
    fn days_to_ymd_year_2000() {
        // 2000-01-01 (Y2K)
        // Unix timestamp 946684800 = 2000-01-01T00:00:00Z
        // days = 946684800 / 86400 = 10957
        let days = 946684800i64 / 86400;
        assert_eq!(days_to_ymd(days), (2000, 1, 1));
    }

    // --- duplicate suffix limit ---

    #[test]
    fn duplicate_returns_error_when_limit_exceeded() {
        let d = tmp();
        create(tdir(&d), "Note", "c").unwrap();
        create(tdir(&d), "Note (copy)", "c").unwrap();
        // Fill in (copy 2) .. (copy 1000)
        for i in 2..=1000u32 {
            std::fs::write(d.path().join(format!("Note (copy {i}).md")), "c").unwrap();
        }
        let err = duplicate(tdir(&d), "Note").unwrap_err();
        assert!(
            matches!(err, TemplatesError::AlreadyExists),
            "expected AlreadyExists when suffix limit exceeded"
        );
    }
}
