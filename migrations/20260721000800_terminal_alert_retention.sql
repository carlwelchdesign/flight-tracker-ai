ALTER TABLE retention_policies
    DROP CONSTRAINT retention_policies_data_class_check,
    ADD CONSTRAINT retention_policies_data_class_check CHECK (
        data_class IN (
            'provider_raw_payload',
            'authorization_audit',
            'session_revocation',
            'identity_mapping',
            'terminal_alert_history'
        )
    );

ALTER TABLE retention_runs
    DROP CONSTRAINT retention_runs_data_class_check,
    ADD CONSTRAINT retention_runs_data_class_check CHECK (
        data_class IN (
            'provider_raw_payload',
            'authorization_audit',
            'session_revocation',
            'identity_mapping',
            'terminal_alert_history'
        )
    );

CREATE TABLE alert_history_tombstones (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    retention_run_id UUID NOT NULL,
    alert_id UUID NOT NULL,
    dedupe_key TEXT NOT NULL CHECK (btrim(dedupe_key) <> ''),
    series_key TEXT NOT NULL CHECK (btrim(series_key) <> ''),
    alert_revision INTEGER NOT NULL CHECK (alert_revision > 0),
    deleted_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, alert_id),
    UNIQUE (operator_id, dedupe_key),
    UNIQUE (operator_id, series_key, alert_revision),
    FOREIGN KEY (operator_id, retention_run_id)
        REFERENCES retention_runs(operator_id, id)
);

CREATE INDEX alert_history_tombstones_run_idx
    ON alert_history_tombstones (operator_id, retention_run_id);

CREATE FUNCTION suppress_tombstoned_alert_history() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM alert_history_tombstones
        WHERE operator_id = NEW.operator_id
          AND (
              alert_id = NEW.id
              OR dedupe_key = NEW.dedupe_key
              OR (series_key = NEW.series_key AND alert_revision = NEW.alert_revision)
          )
    ) THEN
        RETURN NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER alert_history_tombstone_suppression
    BEFORE INSERT ON alerts
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_alert_history();
