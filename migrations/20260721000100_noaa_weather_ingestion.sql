CREATE TABLE airport_observations (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    source_envelope_id UUID NOT NULL,
    schema_version SMALLINT NOT NULL CHECK (schema_version > 0),
    event_time TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL,
    station_code TEXT NOT NULL CHECK (station_code <> ''),
    report_type TEXT NOT NULL CHECK (report_type <> ''),
    raw_text TEXT NOT NULL CHECK (raw_text <> ''),
    provider_received_at TIMESTAMPTZ NOT NULL,
    position GEOMETRY(POINT, 4326) NOT NULL,
    wind_direction_true_degrees DOUBLE PRECISION CHECK (
        wind_direction_true_degrees >= 0 AND wind_direction_true_degrees < 360
    ),
    wind_speed_knots DOUBLE PRECISION CHECK (wind_speed_knots >= 0),
    wind_gust_knots DOUBLE PRECISION CHECK (wind_gust_knots >= 0),
    visibility_statute_miles DOUBLE PRECISION CHECK (visibility_statute_miles >= 0),
    visibility_greater_than BOOLEAN NOT NULL,
    ceiling_feet_agl INTEGER CHECK (ceiling_feet_agl >= 0),
    flight_category TEXT NOT NULL CHECK (
        flight_category IN ('visual', 'marginal_visual', 'instrument', 'low_instrument', 'unknown')
    ),
    UNIQUE (operator_id, id),
    UNIQUE (operator_id, source_envelope_id),
    FOREIGN KEY (operator_id, source_envelope_id)
        REFERENCES provider_envelopes(operator_id, id),
    CHECK (processed_at >= received_at),
    CHECK (ST_X(position) BETWEEN -180 AND 180),
    CHECK (ST_Y(position) BETWEEN -90 AND 90)
);

CREATE INDEX airport_observations_station_time_idx
    ON airport_observations (operator_id, station_code, event_time DESC);
CREATE INDEX airport_observations_position_gist_idx
    ON airport_observations USING GIST (position);

ALTER TABLE weather_hazards
    ADD COLUMN external_series_id TEXT,
    ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    ADD COLUMN supersedes_id UUID,
    ADD COLUMN status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'cancelled')),
    ADD COLUMN issued_at TIMESTAMPTZ,
    ADD COLUMN provider_received_at TIMESTAMPTZ;

UPDATE weather_hazards
SET external_series_id = id::text,
    issued_at = event_time
WHERE external_series_id IS NULL OR issued_at IS NULL;

ALTER TABLE weather_hazards
    ALTER COLUMN external_series_id SET NOT NULL,
    ALTER COLUMN issued_at SET NOT NULL,
    ADD CONSTRAINT weather_hazards_series_revision_unique
        UNIQUE (operator_id, external_series_id, revision),
    ADD CONSTRAINT weather_hazards_supersedes_fk
        FOREIGN KEY (operator_id, supersedes_id)
        REFERENCES weather_hazards(operator_id, id);

ALTER TABLE source_health
    ADD COLUMN last_attempt_at TIMESTAMPTZ,
    ADD COLUMN newest_event_at TIMESTAMPTZ,
    ADD COLUMN consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK (consecutive_failures >= 0);

UPDATE source_health
SET last_attempt_at = observed_at
WHERE last_attempt_at IS NULL;

ALTER TABLE source_health
    ALTER COLUMN last_attempt_at SET NOT NULL;

CREATE TABLE ingestion_failures (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    source_envelope_id UUID NOT NULL,
    error_code TEXT NOT NULL CHECK (error_code <> ''),
    error_detail TEXT NOT NULL CHECK (error_detail <> ''),
    occurred_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, id),
    UNIQUE (operator_id, source_envelope_id),
    FOREIGN KEY (operator_id, source_envelope_id)
        REFERENCES provider_envelopes(operator_id, id)
);
