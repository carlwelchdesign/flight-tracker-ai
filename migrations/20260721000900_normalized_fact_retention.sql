ALTER TABLE retention_policies
    DROP CONSTRAINT retention_policies_data_class_check,
    ADD CONSTRAINT retention_policies_data_class_check CHECK (
        data_class IN (
            'provider_raw_payload',
            'authorization_audit',
            'session_revocation',
            'identity_mapping',
            'terminal_alert_history',
            'normalized_operational_fact'
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
            'terminal_alert_history',
            'normalized_operational_fact'
        )
    );

CREATE TABLE operational_fact_tombstones (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    retention_run_id UUID NOT NULL,
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    fact_type TEXT NOT NULL CHECK (
        fact_type IN (
            'airport_observations',
            'flights',
            'aircraft_positions',
            'planned_routes',
            'weather_hazards'
        )
    ),
    record_id UUID NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL,
    UNIQUE (operator_id, fact_type, record_id),
    FOREIGN KEY (operator_id, retention_run_id)
        REFERENCES retention_runs(operator_id, id)
);

CREATE INDEX operational_fact_tombstones_run_idx
    ON operational_fact_tombstones (operator_id, retention_run_id);

CREATE FUNCTION suppress_tombstoned_operational_fact() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM operational_fact_tombstones
        WHERE operator_id = NEW.operator_id
          AND fact_type = TG_TABLE_NAME
          AND record_id = NEW.id
    ) THEN
        RETURN NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER airport_observation_tombstone_suppression
    BEFORE INSERT ON airport_observations
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_operational_fact();
CREATE TRIGGER flight_tombstone_suppression
    BEFORE INSERT ON flights
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_operational_fact();
CREATE TRIGGER aircraft_position_tombstone_suppression
    BEFORE INSERT ON aircraft_positions
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_operational_fact();
CREATE TRIGGER planned_route_tombstone_suppression
    BEFORE INSERT ON planned_routes
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_operational_fact();
CREATE TRIGGER weather_hazard_tombstone_suppression
    BEFORE INSERT ON weather_hazards
    FOR EACH ROW EXECUTE FUNCTION suppress_tombstoned_operational_fact();

CREATE FUNCTION eligible_normalized_fact_ids(
    scope_operator_id UUID,
    scope_provider TEXT,
    scope_cutoff_at TIMESTAMPTZ
) RETURNS TABLE (fact_type TEXT, record_id UUID)
LANGUAGE SQL STABLE AS $$
    WITH eligible_flights AS (
        SELECT flight.id
        FROM flights flight
        JOIN provider_envelopes envelope
          ON envelope.operator_id = flight.operator_id
         AND envelope.id = flight.source_envelope_id
        WHERE flight.operator_id = scope_operator_id
          AND envelope.provider = scope_provider
          AND flight.status IN ('landed', 'cancelled')
          AND GREATEST(
              flight.event_time,
              flight.received_at,
              flight.processed_at,
              COALESCE(flight.scheduled_arrival_at, flight.processed_at)
          ) < scope_cutoff_at
          AND NOT EXISTS (
              SELECT 1 FROM alerts alert
              WHERE alert.operator_id = flight.operator_id
                AND alert.flight_id = flight.id
          )
          AND NOT EXISTS (
              SELECT 1
              FROM aircraft_positions position
              JOIN provider_envelopes position_envelope
                ON position_envelope.operator_id = position.operator_id
               AND position_envelope.id = position.source_envelope_id
              WHERE position.operator_id = flight.operator_id
                AND position.flight_id = flight.id
                AND (
                    position_envelope.provider <> scope_provider
                    OR GREATEST(position.event_time, position.received_at, position.processed_at)
                       >= scope_cutoff_at
                )
          )
          AND NOT EXISTS (
              SELECT 1
              FROM planned_routes route
              JOIN provider_envelopes route_envelope
                ON route_envelope.operator_id = route.operator_id
               AND route_envelope.id = route.source_envelope_id
              WHERE route.operator_id = flight.operator_id
                AND route.flight_id = flight.id
                AND (
                    route_envelope.provider <> scope_provider
                    OR GREATEST(
                        route.event_time,
                        route.received_at,
                        route.processed_at,
                        route.effective_from,
                        COALESCE(route.effective_to, route.processed_at)
                    ) >= scope_cutoff_at
                )
          )
    ), eligible_hazard_series AS (
        SELECT hazard.external_series_id
        FROM weather_hazards hazard
        JOIN provider_envelopes envelope
          ON envelope.operator_id = hazard.operator_id
         AND envelope.id = hazard.source_envelope_id
        WHERE hazard.operator_id = scope_operator_id
        GROUP BY hazard.external_series_id
        HAVING BOOL_AND(envelope.provider = scope_provider)
           AND MAX(GREATEST(
               hazard.event_time,
               hazard.received_at,
               hazard.processed_at,
               hazard.valid_to,
               hazard.issued_at,
               COALESCE(hazard.provider_received_at, hazard.processed_at)
           )) < scope_cutoff_at
           AND NOT EXISTS (
               SELECT 1
               FROM weather_hazards referenced_hazard
               JOIN alerts alert
                 ON alert.operator_id = referenced_hazard.operator_id
                AND alert.hazard_id = referenced_hazard.id
               WHERE referenced_hazard.operator_id = scope_operator_id
                 AND referenced_hazard.external_series_id = hazard.external_series_id
           )
    )
    SELECT 'airport_observations', observation.id
    FROM airport_observations observation
    JOIN provider_envelopes envelope
      ON envelope.operator_id = observation.operator_id
     AND envelope.id = observation.source_envelope_id
    WHERE observation.operator_id = scope_operator_id
      AND envelope.provider = scope_provider
      AND GREATEST(
          observation.event_time,
          observation.received_at,
          observation.processed_at,
          observation.provider_received_at
      ) < scope_cutoff_at
    UNION ALL
    SELECT 'aircraft_positions', position.id
    FROM aircraft_positions position
    JOIN eligible_flights flight ON flight.id = position.flight_id
    WHERE position.operator_id = scope_operator_id
    UNION ALL
    SELECT 'planned_routes', route.id
    FROM planned_routes route
    JOIN eligible_flights flight ON flight.id = route.flight_id
    WHERE route.operator_id = scope_operator_id
    UNION ALL
    SELECT 'flights', flight.id FROM eligible_flights flight
    UNION ALL
    SELECT 'weather_hazards', hazard.id
    FROM weather_hazards hazard
    JOIN eligible_hazard_series series
      ON series.external_series_id = hazard.external_series_id
    WHERE hazard.operator_id = scope_operator_id;
$$;
