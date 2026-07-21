CREATE TABLE retention_schedules (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    policy_id UUID NOT NULL,
    policy_version INTEGER NOT NULL CHECK (policy_version > 0),
    cadence_seconds BIGINT NOT NULL CHECK (cadence_seconds BETWEEN 3600 AND 2678400),
    status TEXT NOT NULL CHECK (status IN ('draft', 'active', 'paused', 'retired')),
    approval_reference TEXT NOT NULL CHECK (btrim(approval_reference) <> ''),
    created_by_identity_id UUID NOT NULL,
    approved_by_identity_id UUID,
    next_run_at TIMESTAMPTZ NOT NULL,
    last_attempt_at TIMESTAMPTZ,
    last_completed_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK (consecutive_failures >= 0),
    last_error_code TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    approved_at TIMESTAMPTZ,
    paused_at TIMESTAMPTZ,
    UNIQUE (operator_id, id),
    FOREIGN KEY (operator_id, policy_id)
        REFERENCES retention_policies(operator_id, id),
    FOREIGN KEY (operator_id, created_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    FOREIGN KEY (operator_id, approved_by_identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    CHECK (approved_by_identity_id IS NULL OR approved_by_identity_id <> created_by_identity_id),
    CHECK (
        (status = 'draft' AND approved_by_identity_id IS NULL AND approved_at IS NULL AND paused_at IS NULL)
        OR (status = 'active' AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND paused_at IS NULL)
        OR (status IN ('paused', 'retired') AND approved_by_identity_id IS NOT NULL AND approved_at IS NOT NULL AND paused_at IS NOT NULL)
    ),
    CHECK (last_error_code IS NULL OR last_error_code ~ '^[a-z0-9_]{1,64}$')
);

CREATE UNIQUE INDEX retention_schedules_one_active_policy_idx
    ON retention_schedules (operator_id, policy_id)
    WHERE status = 'active';

CREATE INDEX retention_schedules_due_idx
    ON retention_schedules (next_run_at, operator_id, id)
    WHERE status = 'active';

CREATE TABLE retention_schedule_attempts (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    schedule_id UUID NOT NULL,
    scheduled_for TIMESTAMPTZ NOT NULL,
    retention_run_id UUID,
    status TEXT NOT NULL CHECK (status IN ('completed', 'failed')),
    error_code TEXT,
    preview_counts JSONB NOT NULL CHECK (jsonb_typeof(preview_counts) = 'object'),
    attempted_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, id),
    UNIQUE (operator_id, schedule_id, scheduled_for),
    FOREIGN KEY (operator_id, schedule_id)
        REFERENCES retention_schedules(operator_id, id),
    FOREIGN KEY (operator_id, retention_run_id)
        REFERENCES retention_runs(operator_id, id),
    CHECK (
        (status = 'completed' AND retention_run_id IS NOT NULL AND error_code IS NULL)
        OR (status = 'failed' AND retention_run_id IS NULL AND error_code ~ '^[a-z0-9_]{1,64}$')
    )
);

CREATE INDEX retention_schedule_attempts_schedule_time_idx
    ON retention_schedule_attempts (operator_id, schedule_id, scheduled_for DESC);
