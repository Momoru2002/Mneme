mod clock;
mod commands;
pub mod core;
mod db;
mod dto;
mod errors;
mod logging;
mod menu;
mod path_safety;
mod perms;
mod rbac;
mod services;
mod snapshots;
mod sync_surface;
mod webserver;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::db::Db;

// Support/diagnostics commands (menu keeper). Thin wrappers that map the shell
// `errors::Error` to a string for the invoke boundary. No React UI calls these
// yet — they back a future native menu / settings "support" surface — but they
// are registered so that surface only has to call `invoke`, not re-wire Rust.
#[tauri::command]
async fn reveal_logs(app: tauri::AppHandle) -> Result<(), String> {
    menu::reveal_logs(app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn copy_diagnostic_bundle(app: tauri::AppHandle) -> Result<String, String> {
    menu::copy_diagnostic_bundle(app)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn uninstall_and_wipe(app: tauri::AppHandle) -> Result<(), String> {
    menu::uninstall_and_wipe(app).map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_macos_privacy_settings() -> Result<(), String> {
    menu::open_privacy_settings().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Phase 1 — filesystem-surface hygiene for ~/.mneme (sync_surface keeper):
            // ensure the dir is 0700, refuse to run inside iCloud/cloud-storage (which
            // corrupts SQLite WAL), drop a .noindex marker, exclude from Time Machine.
            // tmutil exclusion is skipped-if-already-excluded + backgrounded so it
            // never blocks startup (see sync_surface::exclude_from_time_machine).
            let mneme_dir = perms::mneme_home();
            perms::ensure_mneme_home_at(&mneme_dir)?;
            sync_surface::run_all(&mneme_dir)?;

            // Phase 2 — structured core log + per-launch UUID (logging keeper). Held in
            // state so the diagnostic-bundle command can stamp the bundle with it.
            // Rotate logs older than 7 days first (faithful to the slice-6 retention);
            // best-effort so it never blocks launch.
            let logs_dir = logging::dirs_logs_dir().join("com.mneme.desktop");
            let _ = logging::rotate_old_logs(&logs_dir, 7);
            let log_ctx = logging::open_core_log(&mneme_dir)?;
            app.manage(log_ctx);

            // Phase 3 — open the SQLite database under ~/.mneme, enforcing the on-disk
            // security boundary (dir 0700 + db file 0600 — SEC-3), behind a Mutex in
            // Tauri state (Punakawan condition 1: the sync-rusqlite / async-command
            // bridge, designed once).
            let conn = db::open_app_db(&mneme_dir)?;

            // Integrity check + due daily snapshot (snapshots keeper) on the live
            // connection — bundled rusqlite, no external sqlite3 binary. A failed
            // integrity check is fatal (refuse to run on a corrupt DB); snapshotting
            // is best-effort.
            if !snapshots::integrity_check(&conn)? {
                return Err(Box::<dyn std::error::Error>::from(format!(
                    "DB integrity check failed for {}",
                    mneme_dir.join("mneme.db").display()
                )));
            }
            snapshots::snapshot_if_due(&conn, &mneme_dir.join("snapshots"))?;

            // Auth was removed — resolve (or create) the single local "owner"
            // identity once at startup so it exists before any command runs.
            // Idempotent: adopts an existing lone user on a dev DB.
            services::owner::ensure_owner(&conn)?;

            // Web mode shares the SAME connection with its localhost server thread,
            // so the Db wraps an Arc<Mutex<_>> (web-mode change, superset of the
            // sharing slice which only ever `.lock()`s it).
            app.manage(Db(Arc::new(Mutex::new(conn))));
            app.manage(commands::web_mode::WebMode::default());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::wells::wells_list,
            commands::wells::wells_get,
            commands::wells::wells_add,
            commands::wells::wells_update,
            commands::wells::wells_remove,
            commands::wells::wells_activate,
            commands::wells::wells_validate,
            commands::wells::wells_browse_host,
            commands::wells::wells_host_home,
            commands::tree::tree_list,
            commands::files::files_read,
            commands::files::files_create,
            commands::files::files_update,
            commands::files::files_remove,
            commands::files::files_rename,
            commands::files::files_move,
            commands::files::files_duplicate,
            commands::folders::folders_create,
            commands::folders::folders_remove,
            commands::folders::folders_rename,
            commands::folders::folders_move,
            commands::search::search_query,
            commands::templates::templates_list,
            commands::templates::templates_get,
            commands::templates::templates_create,
            commands::templates::templates_update,
            commands::templates::templates_remove,
            commands::templates::templates_duplicate,
            commands::templates::templates_apply,
            commands::templates::templates_import_defaults,
            reveal_logs,
            copy_diagnostic_bundle,
            uninstall_and_wipe,
            open_macos_privacy_settings,
            commands::web_mode::web_mode_enable,
            commands::web_mode::web_mode_disable,
            commands::web_mode::web_mode_status,
            commands::web_mode::web_mode_open_browser,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // W8: when the app exits, stop any running web-mode server so the
            // accept thread is joined cleanly and the port is released.
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                if let Some(web) = app_handle.try_state::<commands::web_mode::WebMode>() {
                    if let Ok(mut guard) = web.0.lock() {
                        if let Some(srv) = guard.take() {
                            crate::webserver::stop(srv);
                        }
                    }
                }
            }
        });
}
