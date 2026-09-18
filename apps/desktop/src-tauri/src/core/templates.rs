//! Transport-agnostic core for template operations.
//!
//! Templates are filesystem-only (stored under `mneme_home()/templates`), so
//! these `pub fn`s take no DB connection — they resolve the templates dir and
//! delegate to `services::templates`, mapping errors via `client_message()`.
//! Both the Tauri command wrappers and the web-mode HTTP handlers call into this
//! module. Templates are owner-local files with no `well_id`, so there is no
//! read-only-mirror guard.

use crate::dto::{
    ApplyTemplateInput, ApplyTemplateResult, ImportDefaultsResult, RemoveResult,
    TemplateCreateInput, TemplateDto, TemplateListResponse, TemplateUpdateInput,
};
use crate::perms::mneme_home;
use crate::services::templates;

fn templates_dir() -> std::path::PathBuf {
    mneme_home().join("templates")
}

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

pub fn templates_list() -> Result<TemplateListResponse, String> {
    let list = templates::list(&templates_dir()).map_err(|e| e.client_message())?;
    Ok(TemplateListResponse { templates: list })
}

pub fn templates_get(name: &str) -> Result<TemplateDto, String> {
    templates::get(&templates_dir(), name).map_err(|e| e.client_message())
}

pub fn templates_create(input: &TemplateCreateInput) -> Result<TemplateDto, String> {
    templates::create(&templates_dir(), &input.name, &input.content).map_err(|e| e.client_message())
}

pub fn templates_update(name: &str, input: &TemplateUpdateInput) -> Result<TemplateDto, String> {
    templates::update(&templates_dir(), name, &input.content).map_err(|e| e.client_message())
}

pub fn templates_remove(name: &str) -> Result<RemoveResult, String> {
    templates::remove(&templates_dir(), name).map_err(|e| e.client_message())?;
    Ok(RemoveResult { ok: true })
}

pub fn templates_duplicate(name: &str) -> Result<TemplateDto, String> {
    templates::duplicate(&templates_dir(), name).map_err(|e| e.client_message())
}

pub fn templates_apply(
    name: &str,
    input: &ApplyTemplateInput,
) -> Result<ApplyTemplateResult, String> {
    templates::apply(&templates_dir(), name, &input.vars).map_err(|e| e.client_message())
}

pub fn templates_import_defaults() -> Result<ImportDefaultsResult, String> {
    let created = templates::import_defaults(&templates_dir()).map_err(|e| e.client_message())?;
    Ok(ImportDefaultsResult { created })
}
