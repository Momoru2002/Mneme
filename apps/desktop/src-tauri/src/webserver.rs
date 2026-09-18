//! Companion HTTP server (web mode).
//!
//! Lifecycle: `start()` binds either `127.0.0.1:0` (loopback-only, default) or
//! `0.0.0.0:0` (opt-in LAN mode — same machine's local network only, e.g. a
//! phone on the same Wi-Fi) — OS picks a free port either way (race-free,
//! W1), mints a 256-bit session token, and spawns a background accept loop.
//! `stop()` unblocks the server and joins the thread (W8).
//! Task 6: static assets from the embedded web bundle are served for non-/api
//! paths; `/` and unknown extensionless paths fall back to index.html (SPA
//! routing).
//!
//! LAN mode (W13): widening the bind address does NOT widen the Host
//! allowlist to a wildcard — `is_loopback_host` still only accepts loopback
//! hostnames, and LAN mode additionally accepts the machine's own LAN IP
//! (captured once at `start()` and baked into the allowlist), never an
//! arbitrary attacker-supplied Host. This keeps W3 (anti DNS-rebinding) intact:
//! a hostile page that rebinds DNS to our LAN IP still can't forge a Host
//! header we don't already expect.

#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/dist"]
struct WebAssets;

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// 8 MiB cap on an /api request body (W12).
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

use rusqlite::Connection;

pub struct RunningServer {
    pub port: u16,
    pub token: String,
    /// `Some(ip)` when bound for LAN access; `None` in the default
    /// loopback-only mode. Exposed so the Tauri command layer can build a
    /// LAN URL (e.g. for a QR code) without re-detecting the interface.
    pub lan_ip: Option<String>,
    server: Arc<tiny_http::Server>,
    handle: Option<JoinHandle<()>>,
}

fn random_token_hex() -> String {
    let mut bytes = [0u8; 32]; // 256-bit
    getrandom::getrandom(&mut bytes).expect("CSPRNG unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Best-effort detection of this machine's LAN IP.
///
/// Uses the classic "connect" a UDP socket to a public address trick: no
/// packet is actually sent (UDP `connect` just picks a local route), it only
/// asks the OS which local interface *would* be used, then reads that back.
/// Falls back to `None` (LAN mode then can't start) if there's no route,
/// e.g. an offline machine with only loopback.
fn detect_lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

/// Bind for web mode and spawn the accept loop. Returns an error string if
/// the OS refuses the bind (W11) or, for `lan: true`, if no LAN interface
/// could be detected.
///
/// - `lan: false` (default) — binds `127.0.0.1:0`, loopback-only, as before.
/// - `lan: true` — binds `0.0.0.0:0` so other devices on the same local
///   network (e.g. a phone on the same Wi-Fi) can reach it. Still requires
///   the same bearer token (W2) and the Host allowlist is widened only to
///   this machine's own detected LAN IP (W13) — never a wildcard.
pub fn start(db: Arc<Mutex<Connection>>, lan: bool) -> Result<RunningServer, String> {
    let lan_ip = if lan {
        Some(detect_lan_ip().ok_or("no local network connection was found")?)
    } else {
        None
    };
    let bind_addr = if lan { "0.0.0.0:0" } else { "127.0.0.1:0" };
    let server = tiny_http::Server::http(bind_addr)
        .map_err(|_| "could not start the web server".to_string())?;
    let port = match server.server_addr() {
        tiny_http::ListenAddr::IP(a) => a.port(),
        #[allow(unreachable_patterns)]
        _ => return Err("unexpected listen address".into()),
    };
    let token = random_token_hex();
    let server = Arc::new(server);
    let (srv_thread, tok_thread, db_thread, lan_ip_thread) =
        (Arc::clone(&server), token.clone(), db, lan_ip);
    let handle = std::thread::spawn(move || {
        accept_loop(&srv_thread, &tok_thread, port, db_thread, lan_ip_thread)
    });
    Ok(RunningServer {
        port,
        token,
        lan_ip: lan_ip.map(|ip| ip.to_string()),
        server,
        handle: Some(handle),
    })
}

/// Stop the accept loop and join the thread (W8).
pub fn stop(mut srv: RunningServer) {
    srv.server.unblock();
    if let Some(h) = srv.handle.take() {
        let _ = h.join();
    }
}

fn accept_loop(
    server: &tiny_http::Server,
    token: &str,
    port: u16,
    db: Arc<Mutex<Connection>>,
    lan_ip: Option<std::net::IpAddr>,
) {
    for request in server.incoming_requests() {
        handle_request(request, token, port, &db, lan_ip);
    }
}

/// Route an authenticated `/api/<cmd>` POST to the matching core function.
///
/// Returns `Ok(json_string)` on success, or `Err((http_status, message))` on
/// failure.  The error message is the sanitized `client_message()` string from
/// the core — never a raw internal detail (W9).
///
/// Security:
///  - W5: Only the explicitly listed in-scope commands are routed. The in-scope
///    set is the LOCAL surface (note CRUD, tree, wells list/get/activate,
///    settings, search, and templates — all no-network, no-keychain). Version
///    history and owner-profile commands don't exist in this app (git-backed
///    history was dropped, see migration V0005) so there's nothing to route
///    for them.
///    Any command not in the match arm (including wells_add, etc.) returns 404 —
///    out-of-scope surfaces stay desktop-only.
///  - W6: Path confinement is enforced inside `core::files` / `core::folders` /
///    `core::tree` via `path_safety::resolve_well_path`.  The dispatch layer
///    does not need its own check — the guard fires on every call.
pub fn dispatch(cmd: &str, body: &str, conn: &Connection) -> Result<String, (u16, String)> {
    fn parse<'a, T: serde::Deserialize<'a>>(body: &'a str) -> Result<T, (u16, String)> {
        serde_json::from_str(body).map_err(|_| (400u16, "bad request body".to_string()))
    }
    fn json<T: serde::Serialize>(v: &T) -> Result<String, (u16, String)> {
        serde_json::to_string(v).map_err(|_| (500u16, "encoding error".to_string()))
    }
    macro_rules! run {
        ($call:expr) => {
            match $call {
                Ok(v) => json(&v),
                Err(e) => Err((400, e)),
            }
        };
    }
    // Unit-result helper: core fns that return Result<(), String>.
    macro_rules! run_unit {
        ($call:expr) => {
            match $call {
                Ok(()) => Ok("null".to_string()),
                Err(e) => Err((400, e)),
            }
        };
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WellPath {
        well_id: String,
        path: String,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WellInput<T> {
        well_id: String,
        input: T,
    }

    #[derive(serde::Deserialize)]
    struct IdArg {
        id: String,
    }

    #[derive(serde::Deserialize)]
    struct PatchArg {
        patch: crate::dto::UserPrefsPatch,
    }

    // --- arg structs for the local search / templates surface ---
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SearchArg {
        well_id: String,
        q: String,
        #[serde(default)]
        include_content: Option<bool>,
        #[serde(default)]
        options: Option<crate::dto::SearchOptions>,
    }

    #[derive(serde::Deserialize)]
    struct NameArg {
        name: String,
    }

    #[derive(serde::Deserialize)]
    struct InputArg<T> {
        input: T,
    }

    #[derive(serde::Deserialize)]
    struct NameInputArg<T> {
        name: String,
        input: T,
    }

    match cmd {
        // --- files ---
        "files_read" => {
            let a: WellPath = parse(body)?;
            run!(crate::core::files::files_read(conn, &a.well_id, &a.path))
        }
        "files_create" => {
            let a: WellInput<crate::dto::FileCreateInput> = parse(body)?;
            run!(crate::core::files::files_create(
                conn,
                &a.well_id,
                &a.input.path,
                &a.input.content
            ))
        }
        "files_update" => {
            let a: WellInput<crate::dto::FileUpdateInput> = parse(body)?;
            run!(crate::core::files::files_update(
                conn,
                &a.well_id,
                &a.input.path,
                &a.input.content,
                a.input.expected_hash.as_deref()
            ))
        }
        "files_remove" => {
            let a: WellPath = parse(body)?;
            run_unit!(crate::core::files::files_remove(conn, &a.well_id, &a.path))
        }
        "files_rename" => {
            let a: WellInput<crate::dto::FileRenameInput> = parse(body)?;
            run!(crate::core::files::files_rename(
                conn,
                &a.well_id,
                &a.input.old_path,
                &a.input.new_path
            ))
        }
        "files_move" => {
            let a: WellInput<crate::dto::FileMoveInput> = parse(body)?;
            run!(crate::core::files::files_move(
                conn,
                &a.well_id,
                &a.input.source_path,
                &a.input.dest_path
            ))
        }
        "files_duplicate" => {
            let a: WellInput<crate::dto::FileDuplicateInput> = parse(body)?;
            run!(crate::core::files::files_duplicate(
                conn,
                &a.well_id,
                &a.input.path
            ))
        }
        // --- folders ---
        "folders_create" => {
            let a: WellInput<crate::dto::FolderCreateInput> = parse(body)?;
            run!(crate::core::folders::folders_create(
                conn, &a.well_id, &a.input
            ))
        }
        "folders_remove" => {
            let a: WellInput<crate::dto::FolderRemoveInput> = parse(body)?;
            run_unit!(crate::core::folders::folders_remove(
                conn, &a.well_id, &a.input
            ))
        }
        "folders_rename" => {
            let a: WellInput<crate::dto::FolderRenameInput> = parse(body)?;
            run!(crate::core::folders::folders_rename(
                conn, &a.well_id, &a.input
            ))
        }
        "folders_move" => {
            let a: WellInput<crate::dto::FolderMoveInput> = parse(body)?;
            run!(crate::core::folders::folders_move(
                conn, &a.well_id, &a.input
            ))
        }
        // --- tree ---
        "tree_list" => {
            let a: WellPath = parse(body)?;
            run!(crate::core::tree::tree_list(conn, &a.well_id, &a.path))
        }
        // --- wells (list / get / activate only) ---
        "wells_list" => {
            run!(crate::core::wells::wells_list(conn))
        }
        "wells_get" => {
            let a: IdArg = parse(body)?;
            run!(crate::core::wells::wells_get(conn, &a.id))
        }
        "wells_activate" => {
            let a: IdArg = parse(body)?;
            run_unit!(crate::core::wells::wells_activate(conn, &a.id))
        }
        // --- settings ---
        "settings_get" => {
            run!(crate::core::settings::settings_get(conn))
        }
        "settings_update" => {
            let a: PatchArg = parse(body)?;
            run!(crate::core::settings::settings_update(conn, &a.patch))
        }
        // --- search (LOCAL filesystem walk, read-only) ---
        "search_query" => {
            let a: SearchArg = parse(body)?;
            run!(crate::core::search::search_query(
                conn,
                &a.well_id,
                &a.q,
                a.include_content.unwrap_or(true),
                &a.options.unwrap_or_default()
            ))
        }
        // --- templates (LOCAL filesystem-only; no DB connection needed) ---
        "templates_list" => {
            run!(crate::core::templates::templates_list())
        }
        "templates_get" => {
            let a: NameArg = parse(body)?;
            run!(crate::core::templates::templates_get(&a.name))
        }
        "templates_create" => {
            let a: InputArg<crate::dto::TemplateCreateInput> = parse(body)?;
            run!(crate::core::templates::templates_create(&a.input))
        }
        "templates_update" => {
            let a: NameInputArg<crate::dto::TemplateUpdateInput> = parse(body)?;
            run!(crate::core::templates::templates_update(&a.name, &a.input))
        }
        "templates_remove" => {
            let a: NameArg = parse(body)?;
            run!(crate::core::templates::templates_remove(&a.name))
        }
        "templates_duplicate" => {
            let a: NameArg = parse(body)?;
            run!(crate::core::templates::templates_duplicate(&a.name))
        }
        "templates_apply" => {
            let a: NameInputArg<crate::dto::ApplyTemplateInput> = parse(body)?;
            run!(crate::core::templates::templates_apply(&a.name, &a.input))
        }
        "templates_import_defaults" => {
            run!(crate::core::templates::templates_import_defaults())
        }
        // W5: everything else (wells_add, wells_remove, etc.) is explicitly NOT
        // routed.
        _ => Err((404, "unknown command".to_string())),
    }
}

/// The single source of truth for the Host allowlist (W3, anti DNS-rebinding
/// / W13, LAN-mode scoping). Used by BOTH the `/api` gate and the static-asset
/// path so the allowlist can never drift between the two.
///
/// `lan_ip` is `None` in default loopback mode (only loopback hostnames are
/// accepted). In LAN mode it's `Some(this machine's own detected IP)`, which
/// is additionally accepted — never a wildcard, never an arbitrary
/// client-supplied value.
fn is_loopback_host(host: &str, port: u16, lan_ip: Option<std::net::IpAddr>) -> bool {
    let loopback = host == format!("127.0.0.1:{port}")
        || host == format!("localhost:{port}")
        || host == "127.0.0.1"
        || host == "localhost";
    if loopback {
        return true;
    }
    match lan_ip {
        Some(ip) => host == format!("{ip}:{port}") || host == ip.to_string(),
        None => false,
    }
}

/// Gate every /api request: POST only (W12 — also blocks simple-form CSRF, since
/// a cross-origin HTML form can't send our required `Authorization` header without
/// a CORS preflight, which we reject), Host must be loopback or (LAN mode) this
/// machine's own LAN IP (anti DNS-rebinding, W3/W13), and the bearer token must
/// match in constant time (W2).
pub fn check_request(
    method: &str,
    host: Option<&str>,
    auth: Option<&str>,
    port: u16,
    token: &str,
    lan_ip: Option<std::net::IpAddr>,
) -> Result<(), (u16, &'static str)> {
    if method != "POST" {
        return Err((405, "method not allowed"));
    }
    let host = host.ok_or((403, "bad host"))?;
    if !is_loopback_host(host, port, lan_ip) {
        return Err((403, "bad host"));
    }
    let presented = auth.and_then(|h| h.strip_prefix("Bearer ")).unwrap_or("");
    if !constant_time_eq(presented.as_bytes(), token.as_bytes()) {
        return Err((401, "missing or invalid token"));
    }
    Ok(())
}

/// Timing-safe byte comparison: the XOR-fold visits every byte (no short-circuit),
/// so response latency can't leak how many leading bytes of the token matched.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn header<'a>(req: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    let name_lower = name.to_ascii_lowercase();
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().to_ascii_lowercase() == name_lower)
        .map(|h| h.value.as_str())
}

fn respond(request: tiny_http::Request, status: u16, body: &str) {
    let code = tiny_http::StatusCode(status);
    let response = tiny_http::Response::from_string(body).with_status_code(code);
    // W4: No Access-Control-Allow-* headers are added anywhere.
    let _ = request.respond(response);
}

fn respond_json(request: tiny_http::Request, status: u16, body: &str) {
    let code = tiny_http::StatusCode(status);
    let response = tiny_http::Response::from_string(body)
        .with_status_code(code)
        .with_header(tiny_http::Header::from_bytes(b"Content-Type", b"application/json").unwrap());
    // W4: No Access-Control-Allow-* headers are added.
    let _ = request.respond(response);
}

/// True if the declared Content-Length exceeds the body cap (W12).
fn body_too_large(content_length: Option<&str>) -> bool {
    content_length
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|n| n > MAX_BODY_BYTES)
}

/// Map a file extension to a Content-Type string.
fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "woff2" => "font/woff2",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Serve a file from the embedded `WebAssets` bundle.
///
/// - `/` → `index.html`
/// - Other paths → strip the leading `/` and look up in the bundle.
/// - Miss → 404 with an empty body.
///
/// SPA fallback: if the path has no file extension (likely a client-side
/// route), serve `index.html` instead of 404.
pub(crate) fn serve_static(path: &str) -> (u16, String, Vec<u8>) {
    let asset_path = if path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    if let Some(file) = WebAssets::get(asset_path) {
        let ctype = content_type_for(asset_path).to_string();
        return (200, ctype, file.data.into_owned());
    }

    // SPA fallback: extensionless paths are client-side routes → serve index.html.
    let has_extension = asset_path.contains('.');
    if !has_extension {
        if let Some(index) = WebAssets::get("index.html") {
            return (
                200,
                "text/html; charset=utf-8".to_string(),
                index.data.into_owned(),
            );
        }
    }

    (404, "text/plain".to_string(), Vec::new())
}

fn respond_static(request: tiny_http::Request, status: u16, ctype: String, body: Vec<u8>) {
    let code = tiny_http::StatusCode(status);
    let response = tiny_http::Response::from_data(body)
        .with_status_code(code)
        .with_header(tiny_http::Header::from_bytes(b"Content-Type", ctype.as_bytes()).unwrap());
    // W4: No Access-Control-Allow-* headers.
    let _ = request.respond(response);
}

fn handle_request(
    mut request: tiny_http::Request,
    token: &str,
    port: u16,
    db: &Arc<Mutex<Connection>>,
    lan_ip: Option<std::net::IpAddr>,
) {
    let method = request.method().as_str();
    let url = request.url().to_string();

    if url.starts_with("/api/") || url == "/api" {
        let host = header(&request, "host");
        let auth = header(&request, "authorization");
        match check_request(method, host, auth, port, token, lan_ip) {
            Err((code, msg)) => {
                respond(request, code, msg);
                return;
            }
            Ok(()) => {
                // Extract command name from URL (strip leading "/api/").
                let cmd = url.trim_start_matches("/api/");
                // W12: reject declared-oversized bodies early with 413.
                if body_too_large(header(&request, "content-length")) {
                    respond(request, 413, "request body too large");
                    return;
                }
                // Read the request body, bounded to MAX_BODY_BYTES (W12).
                // Handles missing/lying Content-Length or chunked transfer.
                // A body truncated at the cap fails JSON parse → 400.
                let mut body = String::new();
                if request
                    .as_reader()
                    .take(MAX_BODY_BYTES as u64)
                    .read_to_string(&mut body)
                    .is_err()
                {
                    respond(request, 400, "could not read request body");
                    return;
                }
                // Lock the DB and dispatch.
                let conn = match db.lock() {
                    Ok(c) => c,
                    Err(_) => {
                        respond(request, 500, "database lock poisoned");
                        return;
                    }
                };
                match dispatch(cmd, &body, &conn) {
                    Ok(json) => respond_json(request, 200, &json),
                    Err((code, msg)) => respond(request, code, &msg),
                }
                return;
            }
        }
    }

    // Non-/api paths: static bundle serving (Task 6).
    //
    // Host must still be in the allowlist (anti DNS-rebinding, W3/W13) but NO
    // token is required — assets are public (they have no secrets embedded).
    let host = header(&request, "host");
    let host_str = host.unwrap_or("");
    if !is_loopback_host(host_str, port, lan_ip) {
        respond(request, 403, "bad host");
        return;
    }

    if method != "GET" {
        respond(request, 405, "method not allowed");
        return;
    }

    let (status, ctype, body) = serve_static(&url);
    respond_static(request, status, ctype, body);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use std::sync::{Arc, Mutex};

    /// Shared test helper: provision an in-memory DB with one owner + one well.
    /// Returns (conn, well_id, TempDir) — keep the TempDir alive for the test
    /// scope (drop == cleanup; do NOT use TempDir::keep()).
    pub(crate) fn test_db_with_well() -> (rusqlite::Connection, String, tempfile::TempDir) {
        use crate::dto::AddWellInput;
        use crate::services::{owner, wells};
        let conn = open_in_memory().unwrap();
        let user_id = owner::ensure_owner(&conn).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let well = wells::add(
            &conn,
            &AddWellInput {
                name: "W".into(),
                path: dir.path().to_string_lossy().into_owned(),
                color_tag: None,
            },
            &user_id,
        )
        .unwrap();
        (conn, well.id, dir)
    }

    #[test]
    fn dispatch_enforces_confinement_and_unrouts_offscope() {
        let (conn, well_id, _dir) = test_db_with_well();
        // W6: path confinement holds through the HTTP dispatch path
        let body =
            format!(r#"{{"wellId":"{well_id}","input":{{"path":"../escape.md","content":"x"}}}}"#);
        let err = dispatch("files_create", &body, &conn).unwrap_err();
        assert_eq!(err.0, 400);
        assert_eq!(err.1, "invalid path");
        // W5: out-of-scope surfaces are NOT routed (404). wells_add is not in
        // the in-scope route set.
        assert_eq!(dispatch("wells_add", "{}", &conn).unwrap_err().0, 404);
    }

    #[test]
    fn dispatch_routes_local_search_templates_and_keeps_offscope_404() {
        // These LOCAL commands (no network, no keychain) must route; out-of-scope
        // commands must STILL 404 (W5 preserved).
        let (conn, well_id, _dir) = test_db_with_well();

        // search_query routes.
        let search_body =
            format!(r#"{{"wellId":"{well_id}","q":"hello","includeContent":true,"options":null}}"#);
        assert!(
            !matches!(dispatch("search_query", &search_body, &conn), Err((404, _))),
            "search_query must be routed"
        );

        // templates_list routes (no args).
        assert!(
            !matches!(dispatch("templates_list", "{}", &conn), Err((404, _))),
            "templates_list must be routed"
        );

        // W5 preserved: out-of-scope surfaces are STILL 404.
        assert_eq!(dispatch("wells_add", "{}", &conn).unwrap_err().0, 404);
    }

    #[test]
    fn start_binds_loopback_and_reports_port() {
        let db = Arc::new(Mutex::new(open_in_memory().unwrap()));
        let srv = start(db, false).unwrap();
        assert!(srv.port > 0, "OS must assign a real port");
        assert_eq!(srv.token.len(), 64, "256-bit token, hex = 64 chars");
        assert!(srv.lan_ip.is_none(), "loopback mode must not report a LAN ip");
        stop(srv);
    }

    #[test]
    fn auth_requires_bearer_token() {
        assert_eq!(
            check_request("POST", Some("127.0.0.1:5"), None, 5, "secret", None),
            Err((401, "missing or invalid token"))
        );
        assert_eq!(
            check_request(
                "POST",
                Some("127.0.0.1:5"),
                Some("Bearer secret"),
                5,
                "secret",
                None
            ),
            Ok(())
        );
        assert_eq!(
            check_request(
                "POST",
                Some("127.0.0.1:5"),
                Some("Bearer wrong"),
                5,
                "secret",
                None
            ),
            Err((401, "missing or invalid token"))
        );
    }

    #[test]
    fn auth_rejects_foreign_host_header() {
        // anti DNS-rebinding (W3)
        assert_eq!(
            check_request(
                "POST",
                Some("evil.example.com"),
                Some("Bearer secret"),
                5,
                "secret",
                None
            ),
            Err((403, "bad host"))
        );
        assert!(check_request(
            "POST",
            Some("localhost:5"),
            Some("Bearer secret"),
            5,
            "secret",
            None
        )
        .is_ok());
    }

    #[test]
    fn auth_rejects_non_post() {
        assert_eq!(
            check_request(
                "GET",
                Some("127.0.0.1:5"),
                Some("Bearer secret"),
                5,
                "secret",
                None
            ),
            Err((405, "method not allowed"))
        );
    }

    #[test]
    fn lan_mode_accepts_own_ip_but_not_arbitrary_host() {
        // W13: LAN mode widens the allowlist ONLY to the detected LAN IP,
        // never a wildcard — a rebound/foreign Host is still rejected.
        let lan_ip: std::net::IpAddr = "192.168.1.42".parse().unwrap();
        assert!(check_request(
            "POST",
            Some("192.168.1.42:5"),
            Some("Bearer secret"),
            5,
            "secret",
            Some(lan_ip)
        )
        .is_ok());
        assert_eq!(
            check_request(
                "POST",
                Some("evil.example.com"),
                Some("Bearer secret"),
                5,
                "secret",
                Some(lan_ip)
            ),
            Err((403, "bad host"))
        );
        // Loopback must still work even while LAN mode is on.
        assert!(check_request(
            "POST",
            Some("127.0.0.1:5"),
            Some("Bearer secret"),
            5,
            "secret",
            Some(lan_ip)
        )
        .is_ok());
    }

    #[test]
    fn serves_index_html() {
        let (status, ctype, body) = serve_static("/");
        assert_eq!(status, 200);
        assert!(ctype.starts_with("text/html"));
        assert!(!body.is_empty());
    }

    #[test]
    fn serves_js_asset_with_correct_content_type() {
        // Find a JS asset from the bundle and serve it.
        let js_file = WebAssets::iter().find(|f| f.ends_with(".js"));
        assert!(
            js_file.is_some(),
            "the embedded bundle must contain at least one .js asset (run `pnpm --filter @mneme/web build`)"
        );
        if let Some(name) = js_file {
            let path = format!("/{name}");
            let (status, ctype, body) = serve_static(&path);
            assert_eq!(status, 200);
            assert_eq!(ctype, "text/javascript");
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn serves_manifest_with_manifest_json_content_type() {
        let manifest_file = WebAssets::iter().find(|f| f.ends_with(".webmanifest"));
        if let Some(name) = manifest_file {
            let path = format!("/{name}");
            let (status, ctype, _body) = serve_static(&path);
            assert_eq!(status, 200);
            assert_eq!(ctype, "application/manifest+json");
        }
        // If the bundle doesn't have one yet (stale build), this test is a no-op
        // rather than a hard failure — the JS-asset test above already enforces
        // that a fresh bundle exists.
    }

    #[test]
    fn serves_spa_fallback_for_extensionless_path() {
        let (status, ctype, body) = serve_static("/some/client-route");
        assert_eq!(status, 200);
        assert!(ctype.starts_with("text/html"));
        assert!(!body.is_empty());
    }

    #[test]
    fn returns_404_for_unknown_asset_with_extension() {
        let (status, _ctype, body) = serve_static("/does-not-exist.png");
        assert_eq!(status, 404);
        assert!(body.is_empty());
    }

    #[test]
    fn rejects_oversized_body_by_content_length() {
        // W12: declared Content-Length above cap → reject with 413.
        assert!(body_too_large(Some(&(MAX_BODY_BYTES + 1).to_string())));
        // Exactly at the cap is allowed; the .take() bound will enforce the
        // hard stop if the body actually arrives.
        assert!(!body_too_large(Some(&MAX_BODY_BYTES.to_string())));
        assert!(!body_too_large(Some("123")));
        // Missing header → defer to .take() bound; do not pre-reject.
        assert!(!body_too_large(None));
        // Unparseable header → treat as absent; do not pre-reject.
        assert!(!body_too_large(Some("garbage")));
    }
}
