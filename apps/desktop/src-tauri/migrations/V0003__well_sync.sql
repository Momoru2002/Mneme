-- V0003: per-well backup (push-only remote sync) metadata. The PAT is NOT here
-- (it lives in the OS keychain). Append-only; no existing column is altered.
CREATE TABLE well_sync (
    well_id           TEXT PRIMARY KEY REFERENCES wells(id) ON DELETE CASCADE,
    remote_url        TEXT NOT NULL,
    push_consented_at INTEGER,
    last_pushed_at    INTEGER,
    last_status       TEXT
);
