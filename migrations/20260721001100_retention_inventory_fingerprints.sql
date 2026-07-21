ALTER TABLE retention_runs
    ADD COLUMN preview_fingerprint TEXT,
    ADD CONSTRAINT retention_runs_preview_fingerprint_check CHECK (
        preview_fingerprint IS NULL
        OR preview_fingerprint ~ '^[0-9a-f]{64}$'
    );

COMMENT ON COLUMN retention_runs.preview_fingerprint IS
    'SHA-256 of the exact eligible record keys at preview time. NULL only for runs created before this migration; legacy pending runs fail closed at execution.';

ALTER TABLE retention_runs
    ADD CONSTRAINT retention_runs_new_fingerprint_required CHECK (
        preview_fingerprint IS NOT NULL
    ) NOT VALID;

ALTER TABLE retention_schedule_attempts
    ADD COLUMN preview_fingerprint TEXT,
    ADD CONSTRAINT retention_schedule_attempts_preview_fingerprint_check CHECK (
        preview_fingerprint IS NULL
        OR preview_fingerprint ~ '^[0-9a-f]{64}$'
    );

COMMENT ON COLUMN retention_schedule_attempts.preview_fingerprint IS
    'SHA-256 of the exact eligible record keys for this scheduled attempt. NULL for historical attempts or failures recorded before a bounded inventory could be fingerprinted.';
