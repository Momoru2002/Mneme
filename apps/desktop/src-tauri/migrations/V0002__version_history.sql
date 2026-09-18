-- Git version history (foundation slice). One non-destructive column on wells:
-- whether the user enabled git-backed snapshots for that well. The .git on disk
-- is the source of truth for history; this flag avoids a filesystem probe on
-- every render and is reconciled with reality on enable/open.
ALTER TABLE wells ADD COLUMN version_history_enabled INTEGER NOT NULL DEFAULT 0;
