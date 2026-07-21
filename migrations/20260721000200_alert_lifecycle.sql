ALTER TABLE alerts
    ADD COLUMN series_key TEXT,
    ADD COLUMN alert_revision INTEGER NOT NULL DEFAULT 1 CHECK (alert_revision > 0),
    ADD COLUMN supersedes_alert_id UUID,
    ADD COLUMN attention_score SMALLINT NOT NULL DEFAULT 0 CHECK (attention_score BETWEEN 0 AND 100),
    ADD COLUMN score_version INTEGER NOT NULL DEFAULT 1 CHECK (score_version > 0),
    ADD COLUMN evidence JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(evidence) = 'object');

UPDATE alerts
SET series_key = dedupe_key
WHERE series_key IS NULL;

ALTER TABLE alerts
    ALTER COLUMN series_key SET NOT NULL,
    ADD CONSTRAINT alerts_series_key_nonempty CHECK (series_key <> ''),
    ADD CONSTRAINT alerts_series_revision_unique UNIQUE (operator_id, series_key, alert_revision),
    ADD CONSTRAINT alerts_supersedes_fk
        FOREIGN KEY (operator_id, supersedes_alert_id) REFERENCES alerts(operator_id, id);

CREATE INDEX alerts_dispatcher_queue_idx
    ON alerts (operator_id, lifecycle, attention_score DESC, event_time DESC);
CREATE INDEX alerts_series_history_idx
    ON alerts (operator_id, series_key, alert_revision DESC);

ALTER TABLE alert_actions
    ADD COLUMN idempotency_key TEXT;

UPDATE alert_actions
SET idempotency_key = id::text
WHERE idempotency_key IS NULL;

ALTER TABLE alert_actions
    ALTER COLUMN idempotency_key SET NOT NULL,
    ADD CONSTRAINT alert_actions_idempotency_nonempty CHECK (idempotency_key <> ''),
    ADD CONSTRAINT alert_actions_idempotency_unique UNIQUE (operator_id, idempotency_key),
    ADD CONSTRAINT alert_actions_dismiss_reason CHECK (
        action <> 'dismiss' OR (comment IS NOT NULL AND btrim(comment) <> '')
    );
