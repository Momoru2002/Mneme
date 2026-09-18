-- V0005: git features removed. Reconcile existing rows so no well is left in a
-- read-only/mirror state that no code services anymore. Schema columns/tables
-- from V0002-V0004 are intentionally LEFT in place (inert); the app no longer
-- reads them.
UPDATE wells SET kind = 'local', read_only = 0, version_history_enabled = 0
  WHERE kind = 'mirror' OR read_only = 1 OR version_history_enabled = 1;
DELETE FROM well_sync;
DELETE FROM mirror;
