ALTER TABLE alerts
    ADD COLUMN workflow_version INTEGER NOT NULL DEFAULT 1 CHECK (workflow_version > 0),
    ADD COLUMN assigned_identity_id UUID,
    ADD COLUMN assigned_at TIMESTAMPTZ,
    ADD COLUMN assigned_by_actor_id TEXT,
    ADD CONSTRAINT alerts_assignment_complete CHECK (
        (assigned_identity_id IS NULL AND assigned_at IS NULL AND assigned_by_actor_id IS NULL)
        OR (
            assigned_identity_id IS NOT NULL
            AND assigned_at IS NOT NULL
            AND assigned_by_actor_id IS NOT NULL
            AND btrim(assigned_by_actor_id) <> ''
        )
    ),
    ADD CONSTRAINT alerts_assigned_identity_fk
        FOREIGN KEY (assigned_identity_id) REFERENCES auth_identities(id);

CREATE INDEX alerts_dispatcher_assignment_idx
    ON alerts (operator_id, assigned_identity_id, lifecycle, event_time DESC);

ALTER TABLE alert_actions
    DROP CONSTRAINT alert_actions_action_check,
    ADD CONSTRAINT alert_actions_action_check CHECK (
        action IN ('acknowledge', 'assign', 'dismiss', 'comment', 'resolve')
    ),
    ADD COLUMN assigned_identity_id UUID REFERENCES auth_identities(id),
    ADD COLUMN dismissal_reason TEXT;

UPDATE alert_actions
SET dismissal_reason = 'other'
WHERE action = 'dismiss';

ALTER TABLE alert_actions
    ADD CONSTRAINT alert_actions_assignment_target CHECK (
        (action = 'assign' AND assigned_identity_id IS NOT NULL)
        OR (action <> 'assign' AND assigned_identity_id IS NULL)
    ),
    ADD CONSTRAINT alert_actions_dismissal_reason CHECK (
        (action = 'dismiss' AND dismissal_reason IN (
            'duplicate_alert',
            'stale_source_data',
            'incorrect_correlation',
            'not_operationally_relevant',
            'other'
        ))
        OR (action <> 'dismiss' AND dismissal_reason IS NULL)
    );

ALTER TABLE alert_actions
    DROP CONSTRAINT alert_actions_dismiss_reason,
    ADD CONSTRAINT alert_actions_dismiss_comment CHECK (
        action <> 'dismiss'
        OR dismissal_reason <> 'other'
        OR (comment IS NOT NULL AND btrim(comment) <> '')
    );
