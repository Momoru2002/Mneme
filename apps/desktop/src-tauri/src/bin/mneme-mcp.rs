//! `mneme-mcp` — a Model Context Protocol server exposing a running Mneme
//! install's Wells to any MCP-capable AI assistant (Claude Desktop, Claude
//! Code, and anything else that speaks the protocol), over stdio.
//!
//! This is a separate `[[bin]]` in the same package as the desktop app (see
//! Cargo.toml), not a second copy of the app: every tool below is a thin
//! wrapper over `mneme_lib::core::*`, the SAME transport-agnostic functions
//! the desktop app's Tauri commands and its Web Mode HTTP handlers call —
//! same path-confinement, same well-existence guards, same error mapping.
//! This is the third transport, not a parallel implementation.
//!
//! Run manually with `mneme-mcp` (built alongside the desktop app), or point
//! an MCP client's config at the built binary's path so it gets launched
//! automatically. This talks to the SAME `~/.mneme/mneme.db` the desktop app
//! uses (SQLite's WAL mode makes that safe to do concurrently), so notes
//! created or edited here show up in the desktop app immediately, and vice
//! versa — there's no separate synced copy of anything.
//!
//! **Debugging a client that won't connect:** an MCP client spawns this with
//! no visible terminal and usually no accessible stderr, so a silent failure
//! here is otherwise undebuggable from the client side. Every startup, every
//! tool call, and every error is appended to `~/.mneme/mneme-mcp.log` — check
//! that file first if a client (Claude Desktop, Claude Code, ...) reports
//! this server as unavailable or shows zero tools.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use rusqlite::Connection;
use serde::Deserialize;

/// Append one timestamped line to `~/.mneme/mneme-mcp.log`. Best-effort: a
/// logging failure must never take down the actual MCP session, so errors
/// here are swallowed (there is nowhere better to report them to).
fn log_line(line: &str) {
    let path = mneme_lib::core::db::mcp_log_path();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{now}] {line}");
    }
}

/// Serialize a `core::` function's success value as pretty JSON text, or (on
/// its already-sanitized `Err(String)`) return a tool-level error result —
/// `is_error: true` in MCP terms, which lets the model see what went wrong
/// and retry/adjust, rather than crashing the whole session.
fn to_tool_result<T: serde::Serialize>(r: Result<T, String>) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
        }
        Err(msg) => {
            log_line(&format!("tool error: {msg}"));
            Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))
        }
    }
}

/// Resolve an optional `well_id` param to a concrete one: pass through if
/// given, otherwise fall back to the desktop app's currently-active Well.
/// Almost every user only has one Well open at a time, so requiring the
/// model to call `list_wells` before every single other call was pure
/// friction — this lets it skip that unless there's more than one Well or it
/// specifically needs a different one.
fn resolve_well_id(conn: &Connection, well_id: Option<String>) -> Result<String, String> {
    match well_id {
        Some(id) if !id.is_empty() => Ok(id),
        _ => {
            let list = mneme_lib::core::wells::wells_list(conn)?;
            list.active_well_id.ok_or_else(|| {
                "no well_id given and no Well is currently active — call list_wells and pass one explicitly".to_string()
            })
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Search text to match against note filenames and content.
    query: String,
    /// Include matching snippets/content in results, not just paths. Defaults to false.
    #[serde(default)]
    include_content: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReadNoteParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path to the note, relative to the Well root (e.g. "todo/today.md").
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListNotesParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Folder path relative to the Well root. Empty string (the default) lists the Well root.
    #[serde(default)]
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateNoteParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path for the new note, relative to the Well root (e.g. "todo/today.md").
    path: String,
    /// Full markdown content of the new note.
    content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct UpdateNoteParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path to the existing note, relative to the Well root.
    path: String,
    /// New full markdown content, replacing the note's current content.
    content: String,
    /// Optional hash from a prior read_note call on this same note. If given
    /// and the note changed since that read, the update is rejected instead
    /// of silently overwriting a concurrent edit — read the note again, then
    /// retry with the new hash.
    #[serde(default)]
    expected_hash: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AppendToNoteParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path to the existing note, relative to the Well root.
    path: String,
    /// Text to add after the note's current content (a newline is inserted
    /// between the existing content and this, unless the note is empty).
    text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeleteNoteParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path to the note to delete, relative to the Well root.
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateFolderParams {
    /// The Well's id, as returned by list_wells. Omit to use the currently active Well.
    #[serde(default)]
    well_id: Option<String>,
    /// Path for the new folder, relative to the Well root (e.g. "projects/2026").
    path: String,
}

#[derive(serde::Serialize)]
struct Deleted {
    deleted: bool,
    well_id: String,
    path: String,
}

#[derive(Clone)]
struct MnemeMcp {
    conn: Arc<Mutex<Connection>>,
    // Populated in `new()` and consumed by the #[tool_handler]-generated
    // call_tool/list_tools methods (via the tool_router field name convention
    // those macros expect) — clippy's dead-code analysis doesn't trace through
    // that macro-generated access, hence the explicit allow.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MnemeMcp {
    fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List every Well (a folder of markdown notes Mneme is tracking) the \
                        user has registered, and which one is currently active. Every other \
                        tool's well_id is optional and defaults to this active Well, so you \
                        only need this up front if there's more than one Well, or you want to \
                        confirm which one is active."
    )]
    fn list_wells(&self) -> Result<CallToolResult, McpError> {
        log_line("list_wells");
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::wells::wells_list(&conn))
    }

    #[tool(
        description = "Search notes by filename and content within a Well. Returns matching \
                        paths (and snippets if include_content is true)."
    )]
    fn search_notes(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("search_notes query={:?}", p.query));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::search::search_query(
                &conn,
                &well_id,
                &p.query,
                p.include_content,
                &Default::default(),
            )
        })())
    }

    #[tool(description = "Read the full markdown content of one note, by path, within a Well.")]
    fn read_note(
        &self,
        Parameters(p): Parameters<ReadNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("read_note path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::files::files_read(&conn, &well_id, &p.path)
        })())
    }

    #[tool(
        description = "List notes and subfolders at a given path within a Well (like `ls`). \
                        Use an empty path (the default) for the Well root."
    )]
    fn list_notes(
        &self,
        Parameters(p): Parameters<ListNotesParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("list_notes path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::tree::tree_list(&conn, &well_id, &p.path)
        })())
    }

    #[tool(
        description = "Create a new note at the given path within a Well. Fails if a note \
                        already exists there — use update_note or append_to_note for an \
                        existing one."
    )]
    fn create_note(
        &self,
        Parameters(p): Parameters<CreateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("create_note path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::files::files_create(&conn, &well_id, &p.path, &p.content)
        })())
    }

    #[tool(
        description = "Overwrite an existing note's full content within a Well. Pass \
                        expected_hash from a prior read_note to avoid clobbering a change \
                        made since (in the desktop app or elsewhere). To just add to a note \
                        rather than replace it, use append_to_note instead."
    )]
    fn update_note(
        &self,
        Parameters(p): Parameters<UpdateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("update_note path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::files::files_update(
                &conn,
                &well_id,
                &p.path,
                &p.content,
                p.expected_hash.as_deref(),
            )
        })())
    }

    #[tool(
        description = "Add text to the end of an existing note, keeping what's already there \
                        — for task lists, daily logs, or running notes where you're adding an \
                        entry rather than rewriting the whole note. Reads the current content, \
                        appends, and writes back with a hash check so a concurrent edit is \
                        never silently lost (retried once automatically if that happens)."
    )]
    fn append_to_note(
        &self,
        Parameters(p): Parameters<AppendToNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("append_to_note path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            // One retry: if a concurrent write landed between our read and our
            // write, re-read the fresh content once and try again rather than
            // failing a routine append over an ordinary race.
            for _ in 0..2 {
                let current = mneme_lib::core::files::files_read(&conn, &well_id, &p.path)?;
                let new_content = if current.content.is_empty() {
                    p.text.clone()
                } else {
                    format!("{}\n{}", current.content, p.text)
                };
                match mneme_lib::core::files::files_update(
                    &conn,
                    &well_id,
                    &p.path,
                    &new_content,
                    Some(&current.hash),
                ) {
                    Ok(resp) => return Ok(resp),
                    Err(_) => continue, // hash mismatch — one more try with fresh content
                }
            }
            Err(format!(
                "could not append to {} — it kept changing concurrently; try again",
                p.path
            ))
        })())
    }

    #[tool(description = "Delete a note within a Well. This cannot be undone.")]
    fn delete_note(
        &self,
        Parameters(p): Parameters<DeleteNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("delete_note path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::files::files_remove(&conn, &well_id, &p.path)?;
            Ok(Deleted {
                deleted: true,
                well_id,
                path: p.path,
            })
        })())
    }

    #[tool(description = "Create a new (empty) folder within a Well.")]
    fn create_folder(
        &self,
        Parameters(p): Parameters<CreateFolderParams>,
    ) -> Result<CallToolResult, McpError> {
        log_line(&format!("create_folder path={:?}", p.path));
        let conn = self.conn.lock().unwrap();
        to_tool_result((|| {
            let well_id = resolve_well_id(&conn, p.well_id)?;
            mneme_lib::core::folders::folders_create_at(&conn, &well_id, &p.path)
        })())
    }

    #[tool(description = "List the user's saved note templates (name and content).")]
    fn list_templates(&self) -> Result<CallToolResult, McpError> {
        log_line("list_templates");
        to_tool_result(mneme_lib::core::templates::templates_list())
    }
}

#[tool_handler(
    name = "mneme",
    instructions = "Mneme is the user's local knowledge vault: folders of plain markdown notes \
                    (Wells), like Obsidian. Every tool's well_id is optional — omit it to use \
                    whichever Well is currently active; call list_wells first only if the user \
                    has more than one Well or you need a specific one. Use search_notes / \
                    read_note / list_notes to find and read context before answering. Use \
                    create_note for a brand new note, append_to_note to add an entry to an \
                    existing one (task lists, logs) without losing what's there, update_note \
                    to replace a note's content outright, and delete_note to remove one. Paths \
                    are always relative to the well root (e.g. \"todo/today.md\"), never \
                    absolute."
)]
impl ServerHandler for MnemeMcp {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log_line("mneme-mcp starting up");
    let conn = match mneme_lib::core::db::open_app_db() {
        Ok(c) => c,
        Err(e) => {
            log_line(&format!("FATAL: failed to open Mneme's database: {e}"));
            return Err(anyhow::anyhow!("failed to open Mneme's database: {e}"));
        }
    };
    log_line("database opened; entering stdio serve loop");
    let service = match MnemeMcp::new(conn).serve(stdio()).await {
        Ok(s) => s,
        Err(e) => {
            log_line(&format!("FATAL: failed to start serving: {e}"));
            return Err(e.into());
        }
    };
    let result = service.waiting().await;
    match &result {
        Ok(_) => log_line("client disconnected; shutting down normally"),
        Err(e) => log_line(&format!("serve loop ended with error: {e}")),
    }
    result?;
    Ok(())
}
