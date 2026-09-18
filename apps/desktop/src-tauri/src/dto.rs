//! Wire DTOs for all desktop commands (wells, files, folders, tree, templates,
//! search, settings). Field names are camelCase to match the existing React
//! contract (`packages/shared/src/types.ts`), so the api-client is a drop-in.

use serde::{Deserialize, Serialize};

/// Distinguish an absent field (`None`) from an explicit `null` (`Some(None)`)
/// so a value can be cleared to NULL via a patch.
fn de_opt_opt_string<'de, D>(d: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(d)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndentStyle {
    Tabs,
    Spaces,
}

/// Editor/UI preferences (matches `userPrefsSchema` in shared/schemas/settings.ts).
/// `#[serde(default)]` lets a stored partial/older JSON merge over the defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserPrefs {
    pub theme: String,
    pub auto_save_ms: u32,
    pub font_size: u8,
    pub tab_width: u8,
    pub line_numbers: bool,
    pub preview_mermaid: bool,
    pub tree_font_size: u8,
    pub word_wrap: bool,
    pub indent_style: IndentStyle,
    pub accent_color: Option<String>,
    pub gold_color: Option<String>,
    /// Kill switch: outbound network is disabled until the user opts in.
    /// Default `false` — egress off by default (mirrors `allowNetwork` in shared).
    pub allow_network: bool,
}

impl Default for UserPrefs {
    fn default() -> Self {
        // mirrors DEFAULT_PREFS in shared/schemas/settings.ts
        Self {
            theme: "wellspring-dark".to_string(),
            auto_save_ms: 2000,
            font_size: 14,
            tab_width: 2,
            line_numbers: true,
            preview_mermaid: true,
            tree_font_size: 13,
            word_wrap: true,
            indent_style: IndentStyle::Spaces,
            accent_color: None,
            gold_color: None,
            allow_network: false,
        }
    }
}

/// Partial update — every field optional (matches `userPrefsPatchSchema`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPrefsPatch {
    pub theme: Option<String>,
    pub auto_save_ms: Option<u32>,
    pub font_size: Option<u8>,
    pub tab_width: Option<u8>,
    pub line_numbers: Option<bool>,
    pub preview_mermaid: Option<bool>,
    pub tree_font_size: Option<u8>,
    pub word_wrap: Option<bool>,
    pub indent_style: Option<IndentStyle>,
    #[serde(default, deserialize_with = "de_opt_opt_string")]
    pub accent_color: Option<Option<String>>,
    #[serde(default, deserialize_with = "de_opt_opt_string")]
    pub gold_color: Option<Option<String>>,
    pub allow_network: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub prefs: UserPrefs,
}

/// A Well = a tracked directory of knowledge (ports `Well` in shared/types.ts).
/// Timestamps are Unix-ms numbers matching the shared/types.ts contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Well {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color_tag: Option<String>,
    pub sort_order: i64,
    pub last_accessed_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellListResponse {
    pub wells: Vec<Well>,
    pub active_well_id: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellValidationResult {
    pub exists: bool,
    pub is_directory: bool,
    pub readable: bool,
    pub is_obsidian_well: bool,
    pub file_count: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostBrowseEntry {
    pub name: String,
    pub path: String,
    pub has_children: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostBrowseResponse {
    pub parent: Option<String>,
    pub path: String,
    pub entries: Vec<HostBrowseEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddWellInput {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub color_tag: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWellInput {
    pub name: Option<String>,
    pub color_tag: Option<String>,
}

// ---------------------------------------------------------------------------
// Folder DTOs (ports folderCreateSchema / renameSchema / moveSchema from
// packages/shared/src/schemas/files.ts)
// ---------------------------------------------------------------------------

/// Input for folders_create. `path` is relative to the well root.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCreateInput {
    pub path: String,
}

/// Response for folders_create.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCreateResponse {
    pub path: String,
}

/// Input for folders_remove. `path` is relative to the well root.
///
/// `confirm` mirrors the slice-6 HTTP contract (`DELETE /api/folders?confirm=true`
/// in `folders.routes.ts:17+72` and `api-client.ts:186-188`).  The Tauri IPC
/// transport uses a JSON body instead of a URL query parameter, but the field
/// name is deliberately kept identical so the webview call site is a 1:1 port.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRemoveInput {
    pub path: String,
    /// When `true`, remove the directory and all its contents recursively
    /// (mirrors `confirm=true` in the HTTP API).
    #[serde(default)]
    pub confirm: bool,
}

/// Input for folders_rename (ports `renameSchema`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRenameInput {
    pub old_path: String,
    pub new_path: String,
}

/// Response for folders_rename.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRenameResponse {
    pub old_path: String,
    pub new_path: String,
}

/// Input for folders_move (ports `moveSchema`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderMoveInput {
    pub source_path: String,
    pub dest_path: String,
}

/// Response for folders_move.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderMoveResponse {
    pub source_path: String,
    pub dest_path: String,
}

// ---------------------------------------------------------------------------
// File DTOs (ports FileContent / SaveFileResponse / renameSchema / moveSchema
// from packages/shared/src/types.ts + schemas/files.ts)
// ---------------------------------------------------------------------------

/// Input for files_create.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCreateInput {
    pub path: String,
    #[serde(default)]
    pub content: String,
}

/// Input for files_update.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUpdateInput {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub expected_hash: Option<String>,
}

/// Input for files_rename.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRenameInput {
    pub old_path: String,
    pub new_path: String,
}

/// Input for files_move.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMoveInput {
    pub source_path: String,
    pub dest_path: String,
}

/// Input for files_duplicate.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDuplicateInput {
    pub path: String,
}

/// Full file read response — ports `FileContent` in shared/types.ts.
/// `mtime` is Unix-ms (Phase-5 alignment); `frontmatter` is a freeform JSON map.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub well_id: String,
    pub path: String,
    /// Raw markdown source (frontmatter NOT stripped).
    pub content: String,
    /// Markdown body (frontmatter stripped).
    pub body: String,
    /// Parsed frontmatter key→value map.
    pub frontmatter: std::collections::HashMap<String, serde_json::Value>,
    pub size: i64,
    /// Unix-ms timestamp.
    pub mtime: i64,
    /// SHA-256 hex of `content`.
    pub hash: String,
}

/// Compact write response — ports `SaveFileResponse` in shared/types.ts.
/// `mtime` is Unix-ms.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileResponse {
    pub path: String,
    pub size: i64,
    /// Unix-ms timestamp.
    pub mtime: i64,
    pub hash: String,
}

/// Response for files_rename.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRenameResponse {
    pub old_path: String,
    pub new_path: String,
}

/// Response for files_move.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMoveResponse {
    pub source_path: String,
    pub dest_path: String,
}

/// One node in a Well's file tree (ports `TreeEntry` in shared/types.ts).
/// `path` is relative to the Well root; `mtime` is Unix-ms (Phase-5 alignment).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String, // "folder" | "file"
    pub size: Option<i64>,
    pub mtime: i64,
    pub has_children: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeResponse {
    pub well_id: String,
    pub path: String,
    pub entries: Vec<TreeEntry>,
}

// ---------------------------------------------------------------------------
// Template DTOs (ports Template / TemplateListEntry / TemplateListResponse /
// ApplyTemplateResult from packages/shared/src/types.ts + schemas/templates.ts)
// ---------------------------------------------------------------------------

/// A single template file (ports `Template` in shared/types.ts).
/// `size` is bytes; `mtime` is Unix-ms (Phase-5 alignment like other timestamps).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub name: String,
    pub path: String,
    pub content: String,
    pub size: i64,
    /// Unix-ms timestamp.
    pub mtime: i64,
}

/// Compact list entry (ports `TemplateListEntry = Pick<Template, 'name'|'size'|'mtime'>`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateListEntry {
    pub name: String,
    pub size: i64,
    /// Unix-ms timestamp.
    pub mtime: i64,
}

/// List response (ports `TemplateListResponse`).
#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<TemplateListEntry>,
}

/// Result of `templates_import_defaults` — wraps the created list in the
/// `{ created: [...] }` envelope matching the slice-6 API contract
/// (templates.routes.ts:129: `{ created: await importDefaults() }`).
#[derive(Debug, Serialize)]
pub struct ImportDefaultsResult {
    pub created: Vec<TemplateListEntry>,
}

/// Result of `templates_remove` — `{ ok: true }` matches the slice-6 API
/// contract (templates.routes.ts:88: `{ ok: true }`).
#[derive(Debug, Serialize)]
pub struct RemoveResult {
    pub ok: bool,
}

/// Result of applying (rendering) a template (ports `ApplyTemplateResult`).
#[derive(Debug, Serialize)]
pub struct ApplyTemplateResult {
    pub rendered: String,
}

/// Input for templates_create (ports `templateCreateSchema`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateCreateInput {
    pub name: String,
    #[serde(default)]
    pub content: String,
}

/// Input for templates_update (ports `templateUpdateSchema`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateUpdateInput {
    pub content: String,
}

/// Input for templates_apply (ports `applyTemplateSchema`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTemplateInput {
    #[serde(default)]
    pub vars: std::collections::HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Search DTOs (ports SearchMatch / SearchResult / SearchResponse from
// packages/shared/src/types.ts and search-service.ts)
// ---------------------------------------------------------------------------

/// One match within a file — either a filename match or a content match with
/// a line/column reference and a surrounding snippet.
/// Ports `SearchMatch` in shared/types.ts:171-173.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SearchMatch {
    /// The query string appears in the file's relative path / name.
    Filename,
    /// The query string appears in the file's content.
    Content {
        line: u32,
        column: u32,
        snippet: String,
    },
}

/// One file that contains at least one match.
/// Ports `SearchResult` in shared/types.ts:175-178.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    /// Relative path from the well root (forward-slash separated).
    pub path: String,
    pub matches: Vec<SearchMatch>,
}

/// Top-level search response — ports `SearchResponse` in shared/types.ts:180-182.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
}

/// Match options for a search query (wire form; camelCase over IPC).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}
