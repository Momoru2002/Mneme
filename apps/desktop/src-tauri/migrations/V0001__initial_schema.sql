-- Mneme slice 7 — initial SQLite schema (Tauri-native pivot).
--
-- Ported from the slice-6 SQLite DDL
-- (apps/desktop/drizzle/sqlite/0000_broken_maggott.sql on the
-- slice-6-sidecar-attempt tag) with two deliberate slice-7 changes:
--   1. the `sessions` table is DROPPED — the session is in-memory tauri::State
--      now (it dies with the process), so a persistent table is dead weight.
--   2. audit_log's `hmac` column is replaced by a SHA-256 `row_hash` (a hash
--      chain — see db::audit), and query indexes are added so the append-only
--      log can be filtered by date/action/actor without a scan. That indexed
--      query shape is the concrete reason SQLite beats flat files here
--      (2026-05-30 Punakawan condition 2).
--
-- Type mapping (Postgres -> SQLite): uuid -> text (UUIDv4 minted in Rust),
-- timestamptz -> integer (Unix epoch milliseconds), boolean -> integer (0/1),
-- jsonb -> text (JSON string), bigserial -> integer PRIMARY KEY AUTOINCREMENT.

CREATE TABLE users (
    id            text PRIMARY KEY NOT NULL,
    username      text NOT NULL,
    email         text,
    password_hash text NOT NULL,
    role          text NOT NULL DEFAULT 'viewer',
    display_name  text NOT NULL,
    theme_pref    text NOT NULL DEFAULT 'dark',
    is_active     integer NOT NULL DEFAULT 1,
    last_login_at integer,
    created_at    integer NOT NULL,
    updated_at    integer NOT NULL
);
CREATE UNIQUE INDEX users_username_unique ON users (username);
CREATE UNIQUE INDEX users_email_unique ON users (email);

CREATE TABLE wells (
    id                 text PRIMARY KEY NOT NULL,
    name               text NOT NULL,
    path               text NOT NULL,
    color_tag          text,
    sort_order         integer NOT NULL DEFAULT 0,
    last_accessed_at   integer NOT NULL,
    created_by_user_id text NOT NULL,
    created_at         integer NOT NULL,
    updated_at         integer NOT NULL,
    FOREIGN KEY (created_by_user_id) REFERENCES users (id) ON UPDATE NO ACTION ON DELETE RESTRICT
);
CREATE UNIQUE INDEX wells_path_unique ON wells (path);

CREATE TABLE active_well (
    user_id    text PRIMARY KEY NOT NULL,
    well_id   text,
    updated_at integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON UPDATE NO ACTION ON DELETE CASCADE,
    FOREIGN KEY (well_id) REFERENCES wells (id) ON UPDATE NO ACTION ON DELETE SET NULL
);

CREATE TABLE user_settings (
    user_id    text PRIMARY KEY NOT NULL,
    prefs      text NOT NULL DEFAULT '{}',
    updated_at integer NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id) ON UPDATE NO ACTION ON DELETE CASCADE
);

-- No FK on audit_log.user_id: an audit entry must survive the deletion of the
-- user it references (forensic value), so the audit log is intentionally
-- decoupled from the users lifecycle.
CREATE TABLE audit_log (
    id            integer PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_id       text,
    action        text NOT NULL,
    resource_type text NOT NULL,
    resource_ref  text NOT NULL,
    meta          text NOT NULL DEFAULT '{}',
    ip            text,
    user_agent    text,
    created_at    integer NOT NULL,
    row_hash      text NOT NULL
);
CREATE INDEX audit_log_created_at_idx ON audit_log (created_at);
CREATE INDEX audit_log_action_idx ON audit_log (action);
CREATE INDEX audit_log_user_id_idx ON audit_log (user_id);
