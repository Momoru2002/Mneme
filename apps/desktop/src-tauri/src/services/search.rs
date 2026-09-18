//! Intra-well full-text search — filenames + `.md` content.
//!
//! Ports `apps/api/src/services/search-service.ts` 1:1, adapting Node async-fs
//! to `std::fs` and the slice-7 service conventions:
//!   - Pure function; no Tauri state captured.
//!   - `SearchError` with `client_message()` that never leaks fs/DB internals.
//!   - Every path is confined to the Well root via `path_safety::resolve_well_path`.
//!   - Hidden files/directories (name starts with `.`) are skipped.
//!   - Symlinks are NOT followed (lstat via `DirEntry::file_type()` for dirs;
//!     `std::fs::symlink_metadata` for the size/mtime guard on files).
//!   - Only `.md` files are scanned.
//!   - Limits mirror the reference implementation:
//!     - MAX_FILE_SIZE = 1 MiB
//!     - MAX_RESULTS = 200
//!     - MAX_MATCHES_PER_FILE = 5
//!     - MAX_FILES_SCANNED = 5 000
//!     - SNIPPET_CONTEXT = 40 chars

use std::path::Path;

use rusqlite::Connection;

use crate::db::error::DbError;
use crate::dto::{SearchMatch, SearchOptions, SearchResponse, SearchResultItem};
use crate::path_safety::{resolve_well_path, PathSafetyError};
use crate::services::wells;

// ---------------------------------------------------------------------------
// Limits (identical to the reference Node implementation)
// ---------------------------------------------------------------------------
const MD_EXT: &str = ".md";
const MAX_FILE_SIZE: u64 = 1_000_000; // 1 MiB
const MAX_RESULTS: usize = 200;
const MAX_MATCHES_PER_FILE: usize = 5;
const SNIPPET_CONTEXT: usize = 40;
const MAX_FILES_SCANNED: usize = 5_000;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("well not found")]
    WellNotFound,
    #[error("invalid path")]
    PathSafety(#[from] PathSafetyError),
    #[error("invalid regular expression: {0}")]
    InvalidRegex(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl SearchError {
    /// A message safe to cross the IPC boundary to the webview.
    pub fn client_message(&self) -> String {
        match self {
            SearchError::WellNotFound => "well not found".into(),
            SearchError::PathSafety(_) => "invalid path".into(),
            SearchError::InvalidRegex(_) => "invalid regular expression".into(),
            SearchError::Db(_) => "internal error".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Walk `.md` files under `dir` and invoke `cb` for each.
///
/// `scanned` tracks how many files have been yielded.  The walk stops when
/// `*scanned >= max_files` **or** when `cb` returns `false` (early-exit signal,
/// mirroring the streaming `break` in the reference Node generator).
///
/// Returns `false` when `cb` signalled early exit (so callers can propagate),
/// `true` when the walk exhausted naturally.
///
/// symlinks are NOT followed — DirEntry::file_type is lstat-based.
fn walk_markdown<F>(dir: &Path, scanned: &mut usize, max_files: usize, cb: &mut F) -> bool
where
    F: FnMut(&Path) -> bool,
{
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return true,
    };
    for entry in rd.flatten() {
        if *scanned >= max_files {
            return true;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) {
            continue;
        }
        // file_type() is lstat-based: symlinks are neither is_dir nor is_file
        let ft = match entry.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        if ft.is_dir() {
            // Propagate early-exit signal from recursive calls.
            if !walk_markdown(&entry.path(), scanned, max_files, cb) {
                return false;
            }
        } else if ft.is_file() && name.ends_with(MD_EXT) {
            *scanned += 1;
            if !cb(&entry.path()) {
                return false;
            }
        }
    }
    true
}

/// Build a short snippet: `SNIPPET_CONTEXT` chars before and after the match,
/// with ellipsis indicators when the context is truncated.
fn make_snippet(line: &str, idx: usize, needle_len: usize) -> String {
    let start = idx.saturating_sub(SNIPPET_CONTEXT);
    let end = (idx + needle_len + SNIPPET_CONTEXT).min(line.len());
    // Work with char boundaries to avoid UTF-8 panics.  We use byte offsets
    // from the character-based find result, which are valid because
    // `str::to_lowercase` returns a new String but we use `.find()` on the
    // lowercased copy — so the byte offsets may differ from the original for
    // multibyte characters.  The service trades perfect column precision for
    // simplicity (the reference Node impl does the same with `indexOf`).
    let head = if start > 0 { "…" } else { "" };
    let tail = if end < line.len() { "…" } else { "" };

    // Clamp to valid char boundaries.
    let s = clamp_to_char_boundary(line, start);
    let e = clamp_to_char_boundary(line, end);
    format!("{head}{}{tail}", &line[s..e])
}

/// Round a byte index down to the nearest valid UTF-8 character boundary.
fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A compiled matcher for one query under the chosen options.
enum Matcher {
    Regex(regex::Regex),
    Literal {
        /// The needle, already lowercased when `case_sensitive` is false.
        needle: String,
        case_sensitive: bool,
        whole_word: bool,
    },
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Word-boundary check for a literal match spanning bytes [start, end) in `s`.
fn is_word_boundary(s: &str, start: usize, end: usize) -> bool {
    let before_ok = start == 0
        || !s[..start]
            .chars()
            .next_back()
            .map(is_word_char)
            .unwrap_or(false);
    let after_ok = end >= s.len() || !s[end..].chars().next().map(is_word_char).unwrap_or(false);
    before_ok && after_ok
}

impl Matcher {
    fn build(query: &str, o: &SearchOptions) -> Result<Self, SearchError> {
        if o.regex {
            let pattern = if o.whole_word {
                format!(r"\b(?:{query})\b")
            } else {
                query.to_string()
            };
            let re = regex::RegexBuilder::new(&pattern)
                .case_insensitive(!o.case_sensitive)
                .build()
                .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;
            Ok(Matcher::Regex(re))
        } else {
            Ok(Matcher::Literal {
                needle: if o.case_sensitive {
                    query.to_string()
                } else {
                    query.to_lowercase()
                },
                case_sensitive: o.case_sensitive,
                whole_word: o.whole_word,
            })
        }
    }

    /// First match in `text` → (start_byte, len_byte), or None.
    fn first_match(&self, text: &str) -> Option<(usize, usize)> {
        match self {
            Matcher::Regex(re) => re.find(text).map(|m| (m.start(), m.end() - m.start())),
            Matcher::Literal {
                needle,
                case_sensitive,
                whole_word,
            } => {
                if needle.is_empty() {
                    return None;
                }
                let hay = if *case_sensitive {
                    std::borrow::Cow::Borrowed(text)
                } else {
                    std::borrow::Cow::Owned(text.to_lowercase())
                };
                let mut from = 0usize;
                while let Some(rel) = hay[from..].find(needle.as_str()) {
                    let idx = from + rel;
                    let end = idx + needle.len();
                    if !whole_word || is_word_boundary(&hay, idx, end) {
                        return Some((idx, needle.len()));
                    }
                    from = idx + hay[idx..].chars().next().map_or(1, char::len_utf8);
                    if from > hay.len() {
                        break;
                    }
                }
                None
            }
        }
    }

    fn is_match(&self, text: &str) -> bool {
        self.first_match(text).is_some()
    }
}

/// Search `well_id` for `query` across filenames and (optionally) `.md` content.
///
/// * `include_content` — when `false`, only filename matches are returned.
///   Defaults to `true` (mirrors `content !== 'false'` in the route).
/// * Returns up to `MAX_RESULTS` results sorted: filename matches first, then
///   by match count descending, then path alphabetically.
pub fn search_well(
    conn: &Connection,
    well_id: &str,
    query: &str,
    include_content: bool,
    options: &SearchOptions,
) -> Result<SearchResponse, SearchError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(SearchResponse { results: vec![] });
    }

    let well = wells::get(conn, well_id)?.ok_or(SearchError::WellNotFound)?;

    // Resolve the well root itself — enforce that the stored path is absolute
    // and not escaped before we walk it.
    let well_abs = resolve_well_path(&well.path, ".")?;

    let matcher = Matcher::build(trimmed, options)?;

    let mut results: Vec<SearchResultItem> = Vec::new();
    let mut scanned: usize = 0;

    // Walk markdown files inline — the callback returns `false` to signal
    // early exit (mirrors the `break` in the Node streaming generator when
    // `results.length >= MAX_RESULTS`).
    let _ = walk_markdown(
        &well_abs,
        &mut scanned,
        MAX_FILES_SCANNED,
        &mut |abs: &Path| {
            if results.len() >= MAX_RESULTS {
                return false; // stop walking
            }

            // Build the relative path (forward-slash separated, portable).
            let rel = abs
                .strip_prefix(&well_abs)
                .unwrap_or(abs)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");

            let mut matches: Vec<SearchMatch> = Vec::new();

            // --- filename match ---
            if matcher.is_match(&rel) {
                matches.push(SearchMatch::Filename);
            }

            // --- content match ---
            if include_content {
                // Use symlink_metadata (lstat) to check the size: we never follow
                // symlinks to read a file outside the well.
                let size = std::fs::symlink_metadata(abs)
                    .ok()
                    .filter(|m| m.file_type().is_file())
                    .map(|m| m.len())
                    .unwrap_or(u64::MAX);

                if size <= MAX_FILE_SIZE {
                    if let Ok(content) = std::fs::read_to_string(abs) {
                        // Strip any YAML frontmatter so the line numbers are
                        // body-relative — matching exactly what the editor shows
                        // (`FileContent::body`). Reuses files.rs's parser so the
                        // body byte layout (and thus line numbering) agrees.
                        let (body, _) = crate::services::files::parse_frontmatter(&content);
                        let mut content_matches: usize = 0;
                        for (i, line) in body.split('\n').enumerate() {
                            let line = line.trim_end_matches('\r');
                            let Some((idx, len)) = matcher.first_match(line) else {
                                continue;
                            };
                            matches.push(SearchMatch::Content {
                                line: (i + 1) as u32,
                                column: (idx + 1) as u32,
                                snippet: make_snippet(line, idx, len),
                            });
                            content_matches += 1;
                            if content_matches >= MAX_MATCHES_PER_FILE {
                                break;
                            }
                        }
                    }
                }
            }

            if !matches.is_empty() {
                results.push(SearchResultItem { path: rel, matches });
            }

            true // continue walking
        },
    );

    // Sort: filename matches first, then by total match count desc, then path asc.
    results.sort_by(|a, b| {
        let a_fn = u8::from(!a.matches.iter().any(|m| matches!(m, SearchMatch::Filename)));
        let b_fn = u8::from(!b.matches.iter().any(|m| matches!(m, SearchMatch::Filename)));
        if a_fn != b_fn {
            return a_fn.cmp(&b_fn);
        }
        if b.matches.len() != a.matches.len() {
            return b.matches.len().cmp(&a.matches.len());
        }
        a.path.cmp(&b.path)
    });

    Ok(SearchResponse { results })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::dto::{AddWellInput, SearchMatch};

    /// Stand up an in-memory DB with one user + a temp Well directory.
    fn make_well(files: &[(&str, &str)]) -> (Connection, String, tempfile::TempDir) {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, created_at, updated_at) \
             VALUES ('u1', 'alice', 'h', 'Alice', 0, 0)",
            [],
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }
        let well = crate::services::wells::add(
            &conn,
            &AddWellInput {
                name: "Test Well".into(),
                path: dir.path().to_string_lossy().into_owned(),
                color_tag: None,
            },
            "u1",
        )
        .unwrap();
        (conn, well.id, dir)
    }

    #[test]
    fn empty_query_returns_empty() {
        let (conn, id, _dir) = make_well(&[]);
        let resp = search_well(
            &conn,
            &id,
            "   ",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert!(resp.results.is_empty());
    }

    #[test]
    fn unknown_well_is_not_found() {
        let (conn, _id, _dir) = make_well(&[]);
        assert!(matches!(
            search_well(
                &conn,
                "no-such-well",
                "hello",
                true,
                &crate::dto::SearchOptions::default()
            ),
            Err(SearchError::WellNotFound)
        ));
    }

    #[test]
    fn finds_filename_match() {
        let (conn, id, _dir) = make_well(&[("meeting-notes.md", "nothing relevant")]);
        let resp = search_well(
            &conn,
            &id,
            "meeting",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].path, "meeting-notes.md");
        assert!(resp.results[0]
            .matches
            .iter()
            .any(|m| matches!(m, SearchMatch::Filename)));
    }

    #[test]
    fn finds_content_match_with_snippet() {
        let (conn, id, _dir) = make_well(&[("notes.md", "line one\nhello world\nline three")]);
        let resp = search_well(
            &conn,
            &id,
            "hello",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 1);
        let m = resp.results[0]
            .matches
            .iter()
            .find(|m| matches!(m, SearchMatch::Content { .. }))
            .unwrap();
        if let SearchMatch::Content {
            line,
            column,
            snippet,
        } = m
        {
            assert_eq!(*line, 2);
            assert_eq!(*column, 1);
            assert!(snippet.contains("hello world"), "snippet: {snippet}");
        } else {
            panic!("expected Content match");
        }
    }

    #[test]
    fn skips_hidden_files() {
        let (conn, id, _dir) = make_well(&[(".hidden.md", "secret"), ("visible.md", "public")]);
        let resp = search_well(
            &conn,
            &id,
            "secret",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert!(resp.results.is_empty(), "hidden file must be skipped");
    }

    #[test]
    fn skips_non_md_files() {
        let (conn, id, _dir) = make_well(&[("data.txt", "findme"), ("note.md", "irrelevant")]);
        let resp = search_well(
            &conn,
            &id,
            "findme",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert!(resp.results.is_empty(), "non-.md file must be skipped");
    }

    #[test]
    fn content_search_disabled() {
        let (conn, id, _dir) = make_well(&[("note.md", "special content here")]);
        // include_content = false — should not find content-only match
        let resp = search_well(
            &conn,
            &id,
            "special",
            false,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert!(resp.results.is_empty(), "content search was disabled");
    }

    #[test]
    fn filename_matches_sorted_before_content_only() {
        let (conn, id, _dir) = make_well(&[
            ("alpha.md", "needle in content"),      // content-only
            ("needle-doc.md", "unrelated content"), // filename match
        ]);
        let resp = search_well(
            &conn,
            &id,
            "needle",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 2);
        // filename match first
        assert_eq!(resp.results[0].path, "needle-doc.md");
        assert_eq!(resp.results[1].path, "alpha.md");
    }

    #[test]
    fn search_is_case_insensitive() {
        let (conn, id, _dir) = make_well(&[("notes.md", "Hello World")]);
        let resp = search_well(
            &conn,
            &id,
            "HELLO",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 1);
    }

    #[test]
    fn respects_max_matches_per_file() {
        // 10 occurrences → only MAX_MATCHES_PER_FILE (5) content matches returned
        let content = (1..=10)
            .map(|i| format!("line {i} needle"))
            .collect::<Vec<_>>()
            .join("\n");
        let (conn, id, _dir) = make_well(&[("many.md", &content)]);
        let resp = search_well(
            &conn,
            &id,
            "needle",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        let content_match_count = resp.results[0]
            .matches
            .iter()
            .filter(|m| matches!(m, SearchMatch::Content { .. }))
            .count();
        assert!(
            content_match_count <= MAX_MATCHES_PER_FILE,
            "got {content_match_count} content matches, expected ≤ {MAX_MATCHES_PER_FILE}"
        );
    }

    #[test]
    fn walks_subdirectories() {
        let (conn, id, _dir) = make_well(&[
            ("sub/deep/note.md", "deep content needle"),
            ("top.md", "other"),
        ]);
        let resp = search_well(
            &conn,
            &id,
            "needle",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].path, "sub/deep/note.md");
    }

    #[test]
    fn skips_hidden_directories() {
        let (conn, id, _dir) = make_well(&[(".obsidian/config.md", "hidden content needle")]);
        let resp = search_well(
            &conn,
            &id,
            "needle",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert!(resp.results.is_empty(), "hidden directory must be skipped");
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        // File uses CRLF line endings (\r\n); the match should be found and the
        // snippet should not contain a trailing \r (the service strips it).
        let content = "first line\r\nhello world\r\nthird line\r\n";
        let (conn, id, _dir) = make_well(&[("crlf.md", content)]);
        let resp = search_well(
            &conn,
            &id,
            "hello",
            true,
            &crate::dto::SearchOptions::default(),
        )
        .unwrap();
        assert_eq!(resp.results.len(), 1);
        let m = resp.results[0]
            .matches
            .iter()
            .find(|m| matches!(m, SearchMatch::Content { .. }))
            .expect("expected a Content match");
        if let SearchMatch::Content { line, snippet, .. } = m {
            assert_eq!(*line, 2, "match should be on line 2");
            assert!(
                !snippet.contains('\r'),
                "snippet must not contain carriage return; got: {snippet:?}"
            );
        }
    }

    #[test]
    fn content_line_is_body_relative_ignoring_frontmatter() {
        // A file with a YAML frontmatter block. The search line for a body match
        // must be relative to the body the editor shows (frontmatter stripped),
        // NOT the raw file. files.rs's parse_frontmatter yields body
        // "hello body line" for this input, so "hello" is on body line 1.
        let (conn, id, _dir) =
            make_well(&[("n.md", "---\ntitle: X\ntags: [a]\n---\nhello body line")]);
        let r = search_well(&conn, &id, "hello", true, &opts(false, false, false)).unwrap();
        let m = r.results[0]
            .matches
            .iter()
            .find(|m| matches!(m, SearchMatch::Content { .. }))
            .unwrap();
        if let SearchMatch::Content { line, .. } = m {
            assert_eq!(*line, 1, "line is body-relative");
        } else {
            panic!("expected a Content match");
        }
    }

    fn opts(case_sensitive: bool, whole_word: bool, regex: bool) -> crate::dto::SearchOptions {
        crate::dto::SearchOptions {
            case_sensitive,
            whole_word,
            regex,
        }
    }

    #[test]
    fn case_sensitive_distinguishes_case() {
        let (conn, id, _dir) = make_well(&[("n.md", "hello world")]);
        // case-sensitive "Hello" must NOT match lowercase content
        let r = search_well(&conn, &id, "Hello", true, &opts(true, false, false)).unwrap();
        assert!(
            r.results.is_empty(),
            "case-sensitive Hello does not match lowercase hello"
        );
        // case-insensitive matches
        let r2 = search_well(&conn, &id, "Hello", true, &opts(false, false, false)).unwrap();
        assert_eq!(r2.results.len(), 1);
    }

    #[test]
    fn whole_word_multibyte_needle_does_not_panic() {
        // "aé bé": the é at byte 1 fails the boundary (preceded by 'a'); advancing
        // must land on a char boundary, not split the 2-byte é (would panic).
        let (conn, id, _dir) = make_well(&[("n.md", "aé bé\né alone")]);
        let r = search_well(&conn, &id, "é", true, &opts(false, true, false)).unwrap();
        let m = r.results[0]
            .matches
            .iter()
            .find(|m| matches!(m, SearchMatch::Content { .. }))
            .unwrap();
        if let SearchMatch::Content { line, .. } = m {
            assert_eq!(
                *line, 2,
                "the standalone é (line 2) is the whole-word match"
            );
        } else {
            panic!("expected a Content match");
        }
    }

    #[test]
    fn whole_word_excludes_substrings() {
        let (conn, id, _dir) = make_well(&[("n.md", "cat category cats")]);
        let r = search_well(&conn, &id, "cat", true, &opts(false, true, false)).unwrap();
        let m = r.results[0]
            .matches
            .iter()
            .find(|m| matches!(m, SearchMatch::Content { .. }))
            .unwrap();
        if let SearchMatch::Content { column, .. } = m {
            assert_eq!(*column, 1, "first whole-word match is the leading 'cat'");
        }
        let none = search_well(&conn, &id, "categ", true, &opts(false, true, false)).unwrap();
        assert!(none.results.is_empty(), "'categ' is not a whole word here");
    }

    #[test]
    fn whole_word_regex_respects_boundaries() {
        let (conn, id, _dir) = make_well(&[("n.md", "cat category")]);
        // \b(?:cat)\b matches the standalone "cat" but not "category"
        let r = search_well(&conn, &id, "cat", true, &opts(false, true, true)).unwrap();
        let content: Vec<_> = r.results[0]
            .matches
            .iter()
            .filter_map(|m| {
                if let SearchMatch::Content { column, .. } = m {
                    Some(*column)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            content,
            vec![1],
            "only the leading standalone 'cat' (column 1) matches"
        );
        let none = search_well(&conn, &id, "categ", true, &opts(false, true, true)).unwrap();
        assert!(none.results.is_empty(), "'categ' is not a whole word");
    }

    #[test]
    fn options_apply_to_filename_match() {
        let (conn, id, _dir) = make_well(&[("Notes.md", "irrelevant")]);
        let cs = search_well(&conn, &id, "notes", true, &opts(true, false, false)).unwrap();
        assert!(
            cs.results.is_empty(),
            "case-sensitive 'notes' doesn't match 'Notes.md'"
        );
        let ci = search_well(&conn, &id, "notes", true, &opts(false, false, false)).unwrap();
        assert_eq!(ci.results.len(), 1, "case-insensitive matches the filename");
    }

    #[test]
    fn regex_mode_matches_pattern() {
        let (conn, id, _dir) = make_well(&[("n.md", "hallo hello hxllo")]);
        let r = search_well(&conn, &id, "h.llo", true, &opts(false, false, true)).unwrap();
        assert_eq!(r.results.len(), 1, "regex h.llo matches the line");
        let r2 = search_well(&conn, &id, "HELLO", true, &opts(false, false, true)).unwrap();
        assert_eq!(
            r2.results.len(),
            1,
            "regex is case-insensitive when case_sensitive=false"
        );
    }

    #[test]
    fn invalid_regex_is_an_error() {
        let (conn, id, _dir) = make_well(&[("n.md", "anything")]);
        let e = search_well(&conn, &id, "(unclosed", true, &opts(false, false, true));
        assert!(
            matches!(e, Err(SearchError::InvalidRegex(_))),
            "bad pattern → InvalidRegex"
        );
        assert_eq!(
            e.unwrap_err().client_message(),
            "invalid regular expression"
        );
    }
}
