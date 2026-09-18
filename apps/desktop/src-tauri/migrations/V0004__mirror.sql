-- V0004: read-only mirror wells. `kind` distinguishes a normal local well from a
-- mirror of an unverified remote; `read_only` is enforced at the command layer.
ALTER TABLE wells ADD COLUMN kind TEXT NOT NULL DEFAULT 'local';
ALTER TABLE wells ADD COLUMN read_only INTEGER NOT NULL DEFAULT 0;
CREATE TABLE mirror (
    well_id            TEXT PRIMARY KEY REFERENCES wells(id) ON DELETE CASCADE,
    remote_url         TEXT NOT NULL,
    source_pin         TEXT,
    last_refresh_ok_at INTEGER,
    last_attempt_at    INTEGER,
    behind_count       INTEGER,
    last_error_kind    TEXT,
    pending_delete     INTEGER NOT NULL DEFAULT 0
);
