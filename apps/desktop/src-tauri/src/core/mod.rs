//! Transport-agnostic core functions.
//!
//! These are called by both the Tauri `#[tauri::command]` wrappers and the
//! HTTP handlers, ensuring command logic and security guards are not
//! duplicated across transports.

pub mod db;
pub mod files;
pub mod folders;
pub mod search;
pub mod settings;
pub mod templates;
pub mod tree;
pub mod wells;
