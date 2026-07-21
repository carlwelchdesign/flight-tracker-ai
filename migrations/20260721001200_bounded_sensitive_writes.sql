ALTER TABLE alert_actions
    ADD CONSTRAINT alert_actions_actor_id_length CHECK (
        char_length(btrim(actor_id)) BETWEEN 1 AND 256
    ),
    ADD CONSTRAINT alert_actions_idempotency_key_length CHECK (
        char_length(btrim(idempotency_key)) BETWEEN 1 AND 128
    ),
    ADD CONSTRAINT alert_actions_comment_length CHECK (
        comment IS NULL
        OR char_length(btrim(comment)) BETWEEN 1 AND 2000
    );

ALTER TABLE alerts
    ADD CONSTRAINT alerts_assigned_by_actor_id_length CHECK (
        assigned_by_actor_id IS NULL
        OR char_length(btrim(assigned_by_actor_id)) BETWEEN 1 AND 256
    );

ALTER TABLE auth_session_revocations
    ADD CONSTRAINT auth_session_revocations_provider_length CHECK (
        char_length(btrim(provider)) BETWEEN 1 AND 64
    ),
    ADD CONSTRAINT auth_session_revocations_session_id_length CHECK (
        char_length(btrim(session_id)) BETWEEN 1 AND 256
    ),
    ADD CONSTRAINT auth_session_revocations_reason_length CHECK (
        char_length(btrim(reason)) BETWEEN 1 AND 500
    );
