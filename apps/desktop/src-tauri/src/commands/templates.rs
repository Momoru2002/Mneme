//! Template commands — thin `#[tauri::command]` wrappers over `core::templates`.
//!
//! Templates are filesystem-only (no DB lock needed). The templates-dir
//! resolution and error mapping live in the core layer so they apply identically
//! to the Tauri IPC AND the web-mode HTTP transport.

use crate::core::templates as core_templates;
use crate::dto::{
    ApplyTemplateInput, ApplyTemplateResult, ImportDefaultsResult, RemoveResult,
    TemplateCreateInput, TemplateDto, TemplateListResponse, TemplateUpdateInput,
};

#[tauri::command]
pub async fn templates_list() -> Result<TemplateListResponse, String> {
    core_templates::templates_list()
}

#[tauri::command]
pub async fn templates_get(name: String) -> Result<TemplateDto, String> {
    core_templates::templates_get(&name)
}

#[tauri::command]
pub async fn templates_create(input: TemplateCreateInput) -> Result<TemplateDto, String> {
    core_templates::templates_create(&input)
}

#[tauri::command]
pub async fn templates_update(
    name: String,
    input: TemplateUpdateInput,
) -> Result<TemplateDto, String> {
    core_templates::templates_update(&name, &input)
}

#[tauri::command]
pub async fn templates_remove(name: String) -> Result<RemoveResult, String> {
    core_templates::templates_remove(&name)
}

#[tauri::command]
pub async fn templates_duplicate(name: String) -> Result<TemplateDto, String> {
    core_templates::templates_duplicate(&name)
}

#[tauri::command]
pub async fn templates_apply(
    name: String,
    input: ApplyTemplateInput,
) -> Result<ApplyTemplateResult, String> {
    core_templates::templates_apply(&name, &input)
}

#[tauri::command]
pub async fn templates_import_defaults() -> Result<ImportDefaultsResult, String> {
    core_templates::templates_import_defaults()
}
