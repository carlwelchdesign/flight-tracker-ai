ALTER TABLE retention_policies
    DROP CONSTRAINT retention_policies_data_class_check,
    ADD CONSTRAINT retention_policies_data_class_check CHECK (
        data_class IN (
            'provider_raw_payload',
            'authorization_audit',
            'session_revocation',
            'identity_mapping'
        )
    );

ALTER TABLE retention_runs
    DROP CONSTRAINT retention_runs_data_class_check,
    ADD CONSTRAINT retention_runs_data_class_check CHECK (
        data_class IN (
            'provider_raw_payload',
            'authorization_audit',
            'session_revocation',
            'identity_mapping'
        )
    );

CREATE TABLE lifecycle_deletion_tombstones (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    retention_run_id UUID NOT NULL,
    data_class TEXT NOT NULL CHECK (
        data_class IN ('authorization_audit', 'session_revocation', 'identity_mapping')
    ),
    record_id UUID NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, data_class, record_id),
    FOREIGN KEY (operator_id, retention_run_id)
        REFERENCES retention_runs(operator_id, id)
);

CREATE INDEX lifecycle_deletion_tombstones_run_idx
    ON lifecycle_deletion_tombstones (operator_id, retention_run_id);

CREATE FUNCTION suppress_tombstoned_authorization_audit() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM lifecycle_deletion_tombstones
        WHERE operator_id = NEW.operator_id
          AND data_class = 'authorization_audit'
          AND record_id = NEW.id
    ) THEN
        RETURN NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER authorization_audit_tombstone_suppression
    BEFORE INSERT ON authorization_audit_events
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_authorization_audit();

CREATE FUNCTION suppress_tombstoned_session_revocation() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM lifecycle_deletion_tombstones
        WHERE operator_id = NEW.operator_id
          AND data_class = 'session_revocation'
          AND record_id = NEW.id
    ) THEN
        RETURN NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER session_revocation_tombstone_suppression
    BEFORE INSERT ON auth_session_revocations
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_session_revocation();

CREATE FUNCTION minimize_tombstoned_identity() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    tombstone lifecycle_deletion_tombstones%ROWTYPE;
BEGIN
    SELECT * INTO tombstone
    FROM lifecycle_deletion_tombstones
    WHERE data_class = 'identity_mapping' AND record_id = NEW.id
    ORDER BY deleted_at DESC
    LIMIT 1;

    IF FOUND THEN
        NEW.subject = 'deleted:' || NEW.id::text;
        NEW.display_name = NULL;
        NEW.disabled_at = tombstone.deleted_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER auth_identity_tombstone_minimization
    BEFORE INSERT OR UPDATE OF subject, display_name, disabled_at ON auth_identities
    FOR EACH ROW EXECUTE FUNCTION minimize_tombstoned_identity();
