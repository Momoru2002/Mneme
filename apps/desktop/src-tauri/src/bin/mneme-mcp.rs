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

use std::sync::{Arc, Mutex};

use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use rusqlite::Connection;
use serde::Deserialize;

/// Serialize a `core::` function's success value as pretty JSON text, or (on
/// its already-sanitized `Err(String)`) return a tool-level error result —
/// `is_error: true` in MCP terms, which lets the model see what went wrong
/// and retry/adjust, rather than crashing the whole session.
fn to_tool_result<T: serde::Serialize>(r: Result<T, String>) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
        Err(msg) => Ok(CallToolResult::error(vec![Content::text(msg)])),
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// The Well's id, as returned by list_wells.
    well_id: String,
    /// Search text to match against note filenames and content.
    query: String,
    /// Include matching snippets/content in results, not just paths. Defaults to false.
    #[serde(default)]
    include_content: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReadNoteParams {
    /// The Well's id, as returned by list_wells.
    well_id: String,
    /// Path to the note, relative to the Well root (e.g. "todo/today.md").
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListNotesParams {
    /// The Well's id, as returned by list_wells.
    well_id: String,
    /// Folder path relative to the Well root. Empty string (the default) lists the Well root.
    #[serde(default)]
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateNoteParams {
    /// The Well's id, as returned by list_wells.
    well_id: String,
    /// Path for the new note, relative to the Well root (e.g. "todo/today.md").
    path: String,
    /// Full markdown content of the new note.
    content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct UpdateNoteParams {
    /// The Well's id, as returned by list_wells.
    well_id: String,
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

#[derive(Clone)]
struct MnemeMcp {
    conn: Arc<Mutex<Connection>>,
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
                        user has registered, and which one is currently active. Call this \
                        first — every other tool needs a well_id from here."
    )]
    fn list_wells(&self) -> Result<CallToolResult, McpError> {
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
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::search::search_query(
            &conn,
            &p.well_id,
            &p.query,
            p.include_content,
            &Default::default(),
        ))
    }

    #[tool(description = "Read the full markdown content of one note, by path, within a Well.")]
    fn read_note(
        &self,
        Parameters(p): Parameters<ReadNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::files::files_read(&conn, &p.well_id, &p.path))
    }

    #[tool(
        description = "List notes and subfolders at a given path within a Well (like `ls`). \
                        Use an empty path (the default) for the Well root."
    )]
    fn list_notes(
        &self,
        Parameters(p): Parameters<ListNotesParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::tree::tree_list(&conn, &p.well_id, &p.path))
    }

    #[tool(
        description = "Create a new note at the given path within a Well. Fails if a note \
                        already exists there — use update_note to change an existing one."
    )]
    fn create_note(
        &self,
        Parameters(p): Parameters<CreateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::files::files_create(
            &conn,
            &p.well_id,
            &p.path,
            &p.content,
        ))
    }

    #[tool(
        description = "Overwrite an existing note's full content within a Well. Pass \
                        expected_hash from a prior read_note to avoid clobbering a change \
                        made since (in the desktop app or elsewhere)."
    )]
    fn update_note(
        &self,
        Parameters(p): Parameters<UpdateNoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let conn = self.conn.lock().unwrap();
        to_tool_result(mneme_lib::core::files::files_update(
            &conn,
            &p.well_id,
            &p.path,
            &p.content,
            p.expected_hash.as_deref(),
        ))
    }

    #[tool(description = "List the user's saved note templates (name and content).")]
    fn list_templates(&self) -> Result<CallToolResult, McpError> {
        to_tool_result(mneme_lib::core::templates::templates_list())
    }
}

#[tool_handler]
impl ServerHandler for MnemeMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "mneme".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Mneme is the user's local knowledge vault: folders of plain markdown notes \
                 (Wells), like Obsidian. Call list_wells first to get a well_id, then use \
                 search_notes / read_note / list_notes to find and read context before \
                 answering, and create_note / update_note to write notes back — e.g. task \
                 lists, summaries, or follow-ups the user asked you to save. Paths are always \
                 relative to the well root (e.g. \"todo/today.md\"), never absolute."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = mneme_lib::core::db::open_app_db()
        .map_err(|e| anyhow::anyhow!("failed to open Mneme's database: {e}"))?;
    let service = MnemeMcp::new(conn).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
