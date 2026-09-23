//! Tells Settings where the bundled `mneme-mcp` sidecar was installed, so the UI
//! can show a ready-to-paste path/command for MCP clients instead of sending
//! users to build it from source.

use std::path::{Path, PathBuf};

use tauri::State;

use crate::auth::AuthState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpBinaryInfo {
    /// Absolute path to the bundled binary, or `None` when it is not next to
    /// the app executable (e.g. a plain `cargo run` dev build).
    pub path: Option<String>,
    /// `true` when the app runs from a location that changes on every launch
    /// (a Linux AppImage mounts itself under a temp dir), so the path above
    /// must not be saved into an MCP client's config.
    pub ephemeral: bool,
}

/// The sidecar sits directly beside the main executable in every bundle format
/// (Windows install dir, macOS `Contents/MacOS`, Linux `/usr/bin`).
fn mcp_binary_in(dir: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "mneme-mcp.exe"
    } else {
        "mneme-mcp"
    };
    let candidate = dir.join(name);
    candidate.is_file().then_some(candidate)
}

#[tauri::command]
pub async fn mcp_binary_info(
    auth: State<'_, std::sync::Arc<AuthState>>,
) -> Result<McpBinaryInfo, String> {
    crate::auth::require_unlocked(&auth)?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let path = exe
        .parent()
        .and_then(mcp_binary_in)
        .map(|p| p.to_string_lossy().into_owned());
    Ok(McpBinaryInfo {
        path,
        ephemeral: std::env::var_os("APPIMAGE").is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_binary_beside_executable() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "mneme-mcp.exe"
        } else {
            "mneme-mcp"
        };
        std::fs::write(dir.path().join(name), b"x").unwrap();
        let found = mcp_binary_in(dir.path()).expect("binary should be found");
        assert_eq!(found, dir.path().join(name));
    }

    #[test]
    fn returns_none_when_binary_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(mcp_binary_in(dir.path()).is_none());
    }
}
