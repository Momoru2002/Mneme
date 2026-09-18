//! Pure business logic, testable without the Tauri runtime. Command handlers
//! in `crate::commands` are thin wrappers over these.

pub mod files;
pub mod folders;
pub mod owner;
pub mod search;
pub mod settings;
pub mod templates;
pub mod tree;
pub mod users;
pub mod wells;
