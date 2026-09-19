use std::sync::Mutex;

use tauri::State;

use crate::db::Db;

/// Runtime handle for the localhost companion server. `None` when web mode is off.
/// In-memory only — never persisted, never auto-started.
#[derive(Default)]
pub struct WebMode(pub Mutex<Option<crate::webserver::RunningServer>>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebModeInfo {
    pub running: bool,
    pub url: Option<String>,
    /// Present only when LAN mode is on: a URL reachable from other devices
    /// on the same local network (e.g. a phone on the same Wi-Fi). `None` in
    /// the default loopback-only mode.
    pub lan_url: Option<String>,
}

/// Enable web mode. `lan: false` (the historical behavior) binds loopback
/// only. `lan: true` also accepts connections from other devices on this
/// machine's local network, still gated by the same bearer token.
#[tauri::command]
pub async fn web_mode_enable(
    lan: bool,
    db: State<'_, Db>,
    web: State<'_, WebMode>,
) -> Result<WebModeInfo, String> {
    let mut guard = web
        .0
        .lock()
        .map_err(|_| "web mode lock poisoned".to_string())?;
    if let Some(s) = guard.as_ref() {
        return Ok(WebModeInfo {
            running: true,
            url: Some(format!("http://127.0.0.1:{}", s.port)),
            lan_url: s.lan_ip.as_ref().map(|ip| format!("http://{ip}:{}", s.port)),
        });
    }
    let db_arc = std::sync::Arc::clone(&db.0);
    let srv = crate::webserver::start(db_arc, lan)?;
    let url = format!("http://127.0.0.1:{}", srv.port);
    let lan_url = srv.lan_ip.as_ref().map(|ip| format!("http://{ip}:{}", srv.port));
    *guard = Some(srv);
    Ok(WebModeInfo {
        running: true,
        url: Some(url),
        lan_url,
    })
}

#[tauri::command]
pub async fn web_mode_disable(web: State<'_, WebMode>) -> Result<(), String> {
    let mut guard = web
        .0
        .lock()
        .map_err(|_| "web mode lock poisoned".to_string())?;
    if let Some(s) = guard.take() {
        crate::webserver::stop(s);
    }
    Ok(())
}

#[tauri::command]
pub async fn web_mode_status(web: State<'_, WebMode>) -> Result<WebModeInfo, String> {
    let guard = web
        .0
        .lock()
        .map_err(|_| "web mode lock poisoned".to_string())?;
    Ok(match guard.as_ref() {
        Some(s) => WebModeInfo {
            running: true,
            url: Some(format!("http://127.0.0.1:{}", s.port)),
            lan_url: s.lan_ip.as_ref().map(|ip| format!("http://{ip}:{}", s.port)),
        },
        None => WebModeInfo {
            running: false,
            url: None,
            lan_url: None,
        },
    })
}

/// Open the companion URL in the OS browser WITH the token in the fragment,
/// built entirely Rust-side so the token never enters the web/JS layer (W7).
///
/// Cross-platform: `open` on macOS, `xdg-open` on Linux (see
/// `menu::open_with_os_default`) — avoids pulling in `tauri-plugin-opener` as
/// an extra dependency.
#[tauri::command]
pub async fn web_mode_open_browser(web: State<'_, WebMode>) -> Result<(), String> {
    let url = {
        let guard = web
            .0
            .lock()
            .map_err(|_| "web mode lock poisoned".to_string())?;
        let s = guard.as_ref().ok_or("web mode is not running")?;
        format!("http://127.0.0.1:{}/#token={}", s.port, s.token)
    };
    crate::menu::open_with_os_default(&url).map_err(|e| format!("could not open the browser: {e}"))
}
