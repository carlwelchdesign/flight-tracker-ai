use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AirportObservation, CanonicalEvent, FlightCategory, GeoPoint, GeoPolygon, HazardSeverity,
        ProviderEnvelope, SourceHealth, SourceHealthState, Speed, WeatherHazard, WeatherHazardId,
        WeatherHazardStatus,
    },
    ingestion::NormalizedEventBatch,
};

use super::{NoaaFactDraft, PreparedNoaaRecord, SigmetDraft};

#[derive(Debug, Clone, PartialEq)]
pub enum PersistedNoaaRecord {
    Applied(Box<NormalizedEventBatch>),
    Duplicate,
    Quarantined { code: &'static str },
}

#[derive(Debug, Error)]
pub enum NoaaStoreError {
    #[error("NOAA persistence failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("SIGMET revision exceeds supported range")]
    RevisionOverflow,
    #[error("cancelled SIGMET has no prior footprint to supersede")]
    MissingPreviousFootprint,
    #[error("stored SIGMET footprint is invalid: {0}")]
    InvalidStoredFootprint(String),
}

#[derive(Clone)]
pub struct NoaaStore {
    database: PgPool,
}

impl NoaaStore {
    pub fn new(database: PgPool) -> Self {
        Self { database }
    }

    pub async fn persist_record(
        &self,
        record: PreparedNoaaRecord,
    ) -> Result<PersistedNoaaRecord, NoaaStoreError> {
        let mut transaction = self.database.begin().await?;
        let inserted = insert_envelope(&mut transaction, &record.envelope).await?;
        if !inserted {
            transaction.rollback().await?;
            return Ok(PersistedNoaaRecord::Duplicate);
        }

        let event = match record.fact {
            Ok(NoaaFactDraft::Metar(observation)) => {
                insert_airport_observation(&mut transaction, &observation).await?;
                CanonicalEvent::AirportObservation(observation)
            }
            Ok(NoaaFactDraft::AirSigmet(draft)) => {
                let hazard = insert_sigmet_revision(&mut transaction, draft).await?;
                CanonicalEvent::WeatherHazard(hazard)
            }
            Err(failure) => {
                insert_failure(
                    &mut transaction,
                    &record.envelope,
                    failure.code,
                    &failure.detail,
                )
                .await?;
                transaction.commit().await?;
                return Ok(PersistedNoaaRecord::Quarantined { code: failure.code });
            }
        };
        transaction.commit().await?;
        Ok(PersistedNoaaRecord::Applied(Box::new(
            NormalizedEventBatch {
                envelope: record.envelope,
                events: vec![event],
            },
        )))
    }

    pub async fn upsert_source_health(&self, health: &SourceHealth) -> Result<(), NoaaStoreError> {
        sqlx::query(
            r#"
            INSERT INTO source_health (
                id, operator_id, schema_version, provider, feed, state, observed_at,
                last_attempt_at, last_success_at, newest_event_at, consecutive_failures,
                delay_seconds, stale_after_seconds, last_error_code
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12, $13, $14
            )
            ON CONFLICT (operator_id, provider, feed) DO UPDATE SET
                id = EXCLUDED.id,
                schema_version = EXCLUDED.schema_version,
                state = EXCLUDED.state,
                observed_at = EXCLUDED.observed_at,
                last_attempt_at = EXCLUDED.last_attempt_at,
                last_success_at = EXCLUDED.last_success_at,
                newest_event_at = EXCLUDED.newest_event_at,
                consecutive_failures = EXCLUDED.consecutive_failures,
                delay_seconds = EXCLUDED.delay_seconds,
                stale_after_seconds = EXCLUDED.stale_after_seconds,
                last_error_code = EXCLUDED.last_error_code
            "#,
        )
        .bind(health.id.as_uuid())
        .bind(health.operator_id.as_uuid())
        .bind(i16::try_from(health.schema_version.get()).expect("schema version fits i16"))
        .bind(&health.provider)
        .bind(&health.feed)
        .bind(source_health_state(health.state))
        .bind(health.observed_at)
        .bind(health.last_attempt_at)
        .bind(health.last_success_at)
        .bind(health.newest_event_at)
        .bind(i32::try_from(health.consecutive_failures).unwrap_or(i32::MAX))
        .bind(health.delay_seconds.map(saturating_i64))
        .bind(saturating_i64(health.stale_after_seconds))
        .bind(&health.last_error_code)
        .execute(&self.database)
        .await?;
        Ok(())
    }
}

async fn insert_envelope(
    transaction: &mut Transaction<'_, Postgres>,
    envelope: &ProviderEnvelope,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, provider_record_id,
            event_time, received_at, processed_at, raw_payload_sha256, raw_payload
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(envelope.id.as_uuid())
    .bind(envelope.operator_id.as_uuid())
    .bind(i16::try_from(envelope.schema_version.get()).expect("schema version fits i16"))
    .bind(&envelope.provider)
    .bind(&envelope.feed)
    .bind(&envelope.provider_record_id)
    .bind(envelope.event_time)
    .bind(envelope.received_at)
    .bind(envelope.processed_at)
    .bind(&envelope.raw_payload_sha256)
    .bind(&envelope.raw_payload)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn insert_airport_observation(
    transaction: &mut Transaction<'_, Postgres>,
    observation: &AirportObservation,
) -> Result<(), sqlx::Error> {
    let position = json!({
        "type": "Point",
        "coordinates": observation.point.as_geojson_position(),
    });
    sqlx::query(
        r#"
        INSERT INTO airport_observations (
            id, operator_id, source_envelope_id, schema_version, event_time,
            received_at, processed_at, station_code, report_type, raw_text,
            provider_received_at, position, wind_direction_true_degrees,
            wind_speed_knots, wind_gust_knots, visibility_statute_miles,
            visibility_greater_than, ceiling_feet_agl, flight_category
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
            ST_SetSRID(ST_GeomFromGeoJSON($12), 4326), $13, $14, $15, $16, $17, $18, $19
        )
        "#,
    )
    .bind(observation.id.as_uuid())
    .bind(observation.operator_id.as_uuid())
    .bind(observation.source.envelope_id.as_uuid())
    .bind(i16::try_from(observation.schema_version.get()).expect("schema version fits i16"))
    .bind(observation.times.event_time)
    .bind(observation.times.received_at)
    .bind(observation.times.processed_at)
    .bind(&observation.station_code)
    .bind(&observation.report_type)
    .bind(&observation.raw_text)
    .bind(observation.provider_received_at)
    .bind(position.to_string())
    .bind(
        observation
            .wind_direction_true_degrees
            .map(Into::<f64>::into),
    )
    .bind(speed_value(observation.wind_speed))
    .bind(speed_value(observation.wind_gust))
    .bind(observation.visibility_statute_miles)
    .bind(observation.visibility_greater_than)
    .bind(observation.ceiling.map(|value| value.value))
    .bind(flight_category(observation.flight_category))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_sigmet_revision(
    transaction: &mut Transaction<'_, Postgres>,
    draft: SigmetDraft,
) -> Result<WeatherHazard, NoaaStoreError> {
    let previous = sqlx::query_as::<_, (Uuid, i32, String)>(
        r#"
        SELECT id, revision, ST_AsGeoJSON(footprint)::text
        FROM weather_hazards
        WHERE operator_id = $1 AND external_series_id = $2
        ORDER BY revision DESC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(draft.operator_id.as_uuid())
    .bind(&draft.external_series_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let revision = previous
        .as_ref()
        .map(|(_, revision, _)| *revision)
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(NoaaStoreError::RevisionOverflow)?;
    let supersedes_id = previous
        .as_ref()
        .map(|(id, _, _)| WeatherHazardId::from_uuid(*id));
    let footprint = match draft.footprint {
        Some(footprint) => footprint,
        None => previous
            .as_ref()
            .map(|(_, _, footprint)| parse_stored_footprint(footprint))
            .transpose()?
            .ok_or(NoaaStoreError::MissingPreviousFootprint)?,
    };
    let id = WeatherHazardId::from_uuid(Uuid::new_v5(
        &draft.source.envelope_id.as_uuid(),
        b"weather-hazard",
    ));
    let hazard = WeatherHazard {
        id,
        operator_id: draft.operator_id,
        schema_version: draft.schema_version,
        source: draft.source,
        times: draft.times,
        external_series_id: draft.external_series_id,
        revision: u32::try_from(revision).map_err(|_| NoaaStoreError::RevisionOverflow)?,
        supersedes_id,
        status: draft.status,
        issued_at: draft.issued_at,
        provider_received_at: draft.provider_received_at,
        hazard_type: draft.hazard_type,
        severity: draft.severity,
        valid_from: draft.valid_from,
        valid_to: draft.valid_to,
        altitude_band: draft.altitude_band,
        footprint,
    };
    let footprint = json!({
        "type": "Polygon",
        "coordinates": [hazard.footprint.exterior.iter().copied().map(|point| point.as_geojson_position()).collect::<Vec<_>>()],
    });
    sqlx::query(
        r#"
        INSERT INTO weather_hazards (
            id, operator_id, source_envelope_id, schema_version, event_time,
            received_at, processed_at, external_series_id, revision, supersedes_id,
            status, issued_at, provider_received_at, hazard_type, severity,
            valid_from, valid_to, altitude_lower_value, altitude_lower_unit,
            altitude_lower_reference, altitude_upper_value, altitude_upper_unit,
            altitude_upper_reference, footprint
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, ST_SetSRID(ST_GeomFromGeoJSON($24), 4326)
        )
        "#,
    )
    .bind(hazard.id.as_uuid())
    .bind(hazard.operator_id.as_uuid())
    .bind(hazard.source.envelope_id.as_uuid())
    .bind(i16::try_from(hazard.schema_version.get()).expect("schema version fits i16"))
    .bind(hazard.times.event_time)
    .bind(hazard.times.received_at)
    .bind(hazard.times.processed_at)
    .bind(&hazard.external_series_id)
    .bind(revision)
    .bind(hazard.supersedes_id.map(|id| id.as_uuid()))
    .bind(hazard_status(hazard.status))
    .bind(hazard.issued_at)
    .bind(hazard.provider_received_at)
    .bind(&hazard.hazard_type)
    .bind(hazard_severity(hazard.severity))
    .bind(hazard.valid_from)
    .bind(hazard.valid_to)
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.lower)
            .map(|value| value.value),
    )
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.lower)
            .map(|value| altitude_unit(value.unit)),
    )
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.lower)
            .map(|value| altitude_reference(value.reference)),
    )
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.upper)
            .map(|value| value.value),
    )
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.upper)
            .map(|value| altitude_unit(value.unit)),
    )
    .bind(
        hazard
            .altitude_band
            .and_then(|band| band.upper)
            .map(|value| altitude_reference(value.reference)),
    )
    .bind(footprint.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(hazard)
}

async fn insert_failure(
    transaction: &mut Transaction<'_, Postgres>,
    envelope: &ProviderEnvelope,
    code: &str,
    detail: &str,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v5(&envelope.id.as_uuid(), b"ingestion-failure");
    sqlx::query(
        r#"
        INSERT INTO ingestion_failures (
            id, operator_id, source_envelope_id, error_code, error_detail, occurred_at
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(envelope.operator_id.as_uuid())
    .bind(envelope.id.as_uuid())
    .bind(code)
    .bind(detail)
    .bind(envelope.received_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn speed_value(speed: Option<Speed>) -> Option<f64> {
    speed.map(|value| value.value)
}

fn source_health_state(value: SourceHealthState) -> &'static str {
    match value {
        SourceHealthState::Healthy => "healthy",
        SourceHealthState::Degraded => "degraded",
        SourceHealthState::Stale => "stale",
        SourceHealthState::Unavailable => "unavailable",
        SourceHealthState::Unknown => "unknown",
    }
}

fn flight_category(value: FlightCategory) -> &'static str {
    match value {
        FlightCategory::Visual => "visual",
        FlightCategory::MarginalVisual => "marginal_visual",
        FlightCategory::Instrument => "instrument",
        FlightCategory::LowInstrument => "low_instrument",
        FlightCategory::Unknown => "unknown",
    }
}

fn hazard_status(value: WeatherHazardStatus) -> &'static str {
    match value {
        WeatherHazardStatus::Active => "active",
        WeatherHazardStatus::Cancelled => "cancelled",
    }
}

fn hazard_severity(value: HazardSeverity) -> &'static str {
    match value {
        HazardSeverity::Advisory => "advisory",
        HazardSeverity::Significant => "significant",
        HazardSeverity::Severe => "severe",
        HazardSeverity::Unknown => "unknown",
    }
}

fn altitude_unit(value: crate::domain::AltitudeUnit) -> &'static str {
    match value {
        crate::domain::AltitudeUnit::Feet => "feet",
        crate::domain::AltitudeUnit::Meters => "meters",
    }
}

fn altitude_reference(value: crate::domain::AltitudeReference) -> &'static str {
    match value {
        crate::domain::AltitudeReference::MeanSeaLevel => "mean_sea_level",
        crate::domain::AltitudeReference::AboveGroundLevel => "above_ground_level",
        crate::domain::AltitudeReference::FlightLevel => "flight_level",
        crate::domain::AltitudeReference::Ellipsoid => "ellipsoid",
    }
}

fn saturating_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn parse_stored_footprint(value: &str) -> Result<GeoPolygon, NoaaStoreError> {
    let value: serde_json::Value = serde_json::from_str(value)
        .map_err(|error| NoaaStoreError::InvalidStoredFootprint(error.to_string()))?;
    let ring = value
        .get("coordinates")
        .and_then(|value| value.as_array())
        .and_then(|rings| rings.first())
        .and_then(|ring| ring.as_array())
        .ok_or_else(|| NoaaStoreError::InvalidStoredFootprint("missing exterior ring".into()))?;
    let exterior = ring
        .iter()
        .map(|position| {
            let coordinates = position.as_array().ok_or_else(|| {
                NoaaStoreError::InvalidStoredFootprint("position must be an array".into())
            })?;
            let longitude = coordinates
                .first()
                .and_then(|value| value.as_f64())
                .ok_or_else(|| {
                    NoaaStoreError::InvalidStoredFootprint("longitude is missing".into())
                })?;
            let latitude = coordinates
                .get(1)
                .and_then(|value| value.as_f64())
                .ok_or_else(|| {
                    NoaaStoreError::InvalidStoredFootprint("latitude is missing".into())
                })?;
            GeoPoint::new(longitude, latitude)
                .map_err(|error| NoaaStoreError::InvalidStoredFootprint(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GeoPolygon { exterior })
}
