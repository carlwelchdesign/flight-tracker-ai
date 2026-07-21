CREATE TABLE retention_policies (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    data_class TEXT NOT NULL CHECK (data_class IN ('provider_raw_payload')),
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    version INTEGER NOT NULL CHECK (version > 0),
    retention_seconds BIGINT NOT NULL CHECK (retention_seconds BETWEEN 3600 AND 315576000),
    status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'retired')),
    approval_reference TEXT NOT NULL CHECK (btrim(approval_reference) <> ''),
    created_by_identity_id UUID NOT NULL,
    approved_by_identity_id UUID,
    created_at TIMESTAMPTZ NOT NULL,
    approved_at TIMESTAMPTZ,
    retired_at TIMESTAMPTZ,
    UNIQUE (operator_id, id),
    UNIQUE (operator_id, data_class, provider, version),
    FOREIGN KEY (operator_id, created_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    FOREIGN KEY (operator_id, approved_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    CHECK (
        (status = 'draft' AND approved_by_identity_id IS NULL AND approved_at IS NULL AND retired_at IS NULL)
        OR (status = 'approved' AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND retired_at IS NULL)
        OR (status = 'retired' AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND retired_at IS NOT NULL)
    ),
    CHECK (approved_by_identity_id IS NULL OR approved_by_identity_id <> created_by_identity_id)
);

CREATE UNIQUE INDEX retention_policies_one_approved_scope_idx
    ON retention_policies (operator_id, data_class, provider)
    WHERE status = 'approved';

CREATE TABLE retention_runs (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    policy_id UUID NOT NULL,
    policy_version INTEGER NOT NULL CHECK (policy_version > 0),
    data_class TEXT NOT NULL CHECK (data_class IN ('provider_raw_payload')),
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    cutoff_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('awaiting_approval', 'approved', 'completed', 'cancelled')),
    preview_counts JSONB NOT NULL CHECK (jsonb_typeof(preview_counts) = 'object'),
    deletion_counts JSONB CHECK (deletion_counts IS NULL OR jsonb_typeof(deletion_counts) = 'object'),
    requested_by_identity_id UUID NOT NULL,
    approved_by_identity_id UUID,
    executed_by_identity_id UUID,
    evidence_reference TEXT NOT NULL CHECK (btrim(evidence_reference) <> ''),
    requested_at TIMESTAMPTZ NOT NULL,
    approved_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    UNIQUE (operator_id, id),
    FOREIGN KEY (operator_id, policy_id)
        REFERENCES retention_policies(operator_id, id),
    FOREIGN KEY (operator_id, requested_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    FOREIGN KEY (operator_id, approved_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    FOREIGN KEY (operator_id, executed_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    CHECK (approved_by_identity_id IS NULL OR approved_by_identity_id <> requested_by_identity_id),
    CHECK (
        (status = 'awaiting_approval' AND approved_by_identity_id IS NULL AND approved_at IS NULL AND executed_by_identity_id IS NULL AND started_at IS NULL AND completed_at IS NULL AND deletion_counts IS NULL)
        OR (status = 'approved' AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND executed_by_identity_id IS NULL AND started_at IS NULL AND completed_at IS NULL AND deletion_counts IS NULL)
        OR (status = 'completed' AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND executed_by_identity_id IS NOT NULL AND started_at IS NOT NULL AND completed_at IS NOT NULL AND deletion_counts IS NOT NULL)
        OR (status = 'cancelled')
    )
);

CREATE INDEX retention_runs_operator_time_idx
    ON retention_runs (operator_id, requested_at DESC, id DESC);

CREATE TABLE data_deletion_tombstones (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    retention_run_id UUID NOT NULL,
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    feed TEXT NOT NULL CHECK (btrim(feed) <> ''),
    source_envelope_id UUID NOT NULL,
    raw_payload_sha256 TEXT NOT NULL CHECK (raw_payload_sha256 ~ '^[0-9a-f]{64}$'),
    deleted_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, provider, feed, raw_payload_sha256),
    FOREIGN KEY (operator_id, retention_run_id)
        REFERENCES retention_runs(operator_id, id)
);

CREATE INDEX data_deletion_tombstones_run_idx
    ON data_deletion_tombstones (operator_id, retention_run_id);

ALTER TABLE provider_envelopes
    ADD COLUMN raw_payload_deleted_at TIMESTAMPTZ,
    ADD COLUMN deletion_tombstone_id UUID;

CREATE INDEX provider_envelopes_retention_eligible_idx
    ON provider_envelopes (operator_id, provider, received_at, id)
    WHERE raw_payload_deleted_at IS NULL;

CREATE FUNCTION suppress_tombstoned_provider_payload() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    tombstone data_deletion_tombstones%ROWTYPE;
BEGIN
    SELECT * INTO tombstone
    FROM data_deletion_tombstones
    WHERE operator_id = NEW.operator_id
      AND provider = NEW.provider
      AND feed = NEW.feed
      AND raw_payload_sha256 = NEW.raw_payload_sha256;

    IF FOUND THEN
        NEW.raw_payload = '{}'::jsonb;
        NEW.raw_payload_deleted_at = tombstone.deleted_at;
        NEW.deletion_tombstone_id = tombstone.id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER provider_envelopes_tombstone_suppression
    BEFORE INSERT OR UPDATE OF raw_payload ON provider_envelopes
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_provider_payload();
