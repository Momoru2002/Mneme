//! Template commands — thin `#[tauri::command]` wrappers over `core::templates`.
//!
//! Templates are filesystem-only (no DB lock needed). The templates-dir
//! resolution and error mapping live in the core layer so they apply identically
//! to the Tauri IPC AND the web-mode HTTP transport.

use tauri::State;

use crate::auth::AuthState;
use crate::core::templates as core_templates;
use crate::dto::{
    ApplyTemplateInput, ApplyTemplateResult, ImportDefaultsResult, RemoveResult,
    TemplateCreateInput, TemplateDto, TemplateListResponse, TemplateUpdateInput,
};

#[tauri::command]
pub async fn templates_list(auth: State<'_, std::sync::Arc<AuthState>>) -> Result<TemplateListResponse, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_list()
}

#[tauri::command]
pub async fn templates_get(
    auth: State<'_, std::sync::Arc<AuthState>>,
    name: String,
) -> Result<TemplateDto, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_get(&name)
}

#[tauri::command]
pub async fn templates_create(
    auth: State<'_, std::sync::Arc<AuthState>>,
    input: TemplateCreateInput,
) -> Result<TemplateDto, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_create(&input)
}

#[tauri::command]
pub async fn templates_update(
    auth: State<'_, std::sync::Arc<AuthState>>,
    name: String,
    input: TemplateUpdateInput,
) -> Result<TemplateDto, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_update(&name, &input)
}

#[tauri::command]
pub async fn templates_remove(
    auth: State<'_, std::sync::Arc<AuthState>>,
    name: String,
) -> Result<RemoveResult, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_remove(&name)
}

#[tauri::command]
pub async fn templates_duplicate(
    auth: State<'_, std::sync::Arc<AuthState>>,
    name: String,
) -> Result<TemplateDto, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_duplicate(&name)
}

#[tauri::command]
pub async fn templates_apply(
    auth: State<'_, std::sync::Arc<AuthState>>,
    name: String,
    input: ApplyTemplateInput,
) -> Result<ApplyTemplateResult, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_apply(&name, &input)
}

#[tauri::command]
pub async fn templates_import_defaults(
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<ImportDefaultsResult, String> {
    crate::auth::require_unlocked(&auth)?;
    core_templates::templates_import_defaults()
}
