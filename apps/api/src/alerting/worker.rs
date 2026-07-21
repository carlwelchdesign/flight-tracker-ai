use std::{collections::HashMap, sync::Arc, time::Duration};

use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::{sync::broadcast, task::JoinHandle};

use crate::{
    domain::{
        AircraftPosition, AltitudeBand, AltitudeReference, AltitudeUnit, CanonicalEvent, Flight,
        FlightId, FlightStatus, HazardSeverity, OperatorId, PlannedRoute, SourceQuality, SpeedUnit,
        WeatherHazard, WeatherHazardStatus,
    },
    health::WorkerProbe,
    ingestion::NormalizedEventBatch,
};

use super::{AlertStore, RouteHazardInput, RouteHazardRule, candidate_from_route_hazard};

#[derive(Default)]
struct CorrelationState {
    routes: HashMap<(OperatorId, FlightId), PlannedRoute>,
    hazards: HashMap<OperatorId, HashMap<String, WeatherHazard>>,
    positions: HashMap<(OperatorId, FlightId), AircraftPosition>,
}

pub fn spawn_alert_worker(
    database: PgPool,
    mut receiver: broadcast::Receiver<Arc<NormalizedEventBatch>>,
    mut probe: WorkerProbe,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let store = AlertStore::new(database.clone());
        let rule = RouteHazardRule::default();
        let mut state = CorrelationState::default();
        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = heartbeat.tick() => probe.heartbeat(),
                received = receiver.recv() => {
                    let batch = match received {
                        Ok(batch) => batch,
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(worker = "alert_projection", skipped, "alert input lagged");
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    };

                    if let Err(error) = persist_simulation_batch(&database, &batch).await {
                        tracing::error!(worker = "alert_projection", error = %error, "canonical replay persistence failed");
                        probe.fail("canonical replay persistence failed");
                        break;
                    }
                    absorb(&mut state, &batch);
                    if let Err(error) = evaluate(&state, &store, &rule, &batch).await {
                        tracing::error!(worker = "alert_projection", error = %error, "alert evaluation failed");
                        probe.fail("alert evaluation failed");
                        break;
                    }
                    probe.heartbeat();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;
    use tokio::{sync::broadcast, time::sleep};

    use super::*;
    use crate::health::CriticalWorkerRegistry;

    #[tokio::test]
    async fn idle_alert_worker_keeps_its_health_heartbeat_current() {
        let database = PgPoolOptions::new()
            .connect_lazy("postgresql://postgres:postgres@127.0.0.1:1/test")
            .unwrap();
        let (sender, receiver) = broadcast::channel(2);
        let workers = CriticalWorkerRegistry::default();
        let handle = spawn_alert_worker(database, receiver, workers.register("alert_projection"));

        sleep(Duration::from_millis(3_200)).await;

        assert!(workers.is_ready());
        handle.abort();
        drop(sender);
    }
}

fn absorb(state: &mut CorrelationState, batch: &NormalizedEventBatch) {
    for event in &batch.events {
        match event {
            CanonicalEvent::PlannedRoute(route) => {
                let key = (route.operator_id, route.flight_id);
                if state
                    .routes
                    .get(&key)
                    .is_none_or(|current| route.route_version >= current.route_version)
                {
                    state.routes.insert(key, route.clone());
                }
            }
            CanonicalEvent::WeatherHazard(hazard) => {
                let hazards = state.hazards.entry(hazard.operator_id).or_default();
                if hazards
                    .get(&hazard.external_series_id)
                    .is_none_or(|current| hazard.revision >= current.revision)
                {
                    hazards.insert(hazard.external_series_id.clone(), hazard.clone());
                }
            }
            CanonicalEvent::AircraftPosition(position) => {
                let key = (position.operator_id, position.flight_id);
                if state
                    .positions
                    .get(&key)
                    .is_none_or(|current| position.times.event_time >= current.times.event_time)
                {
                    state.positions.insert(key, position.clone());
                }
            }
            _ => {}
        }
    }
}

async fn evaluate(
    state: &CorrelationState,
    store: &AlertStore,
    rule: &RouteHazardRule,
    batch: &NormalizedEventBatch,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let evaluated_at = batch
        .envelope
        .event_time
        .unwrap_or(batch.envelope.received_at);
    for ((operator_id, flight_id), route) in &state.routes {
        let Some(hazards) = state.hazards.get(operator_id) else {
            continue;
        };
        let altitude = state
            .positions
            .get(&(*operator_id, *flight_id))
            .and_then(|position| position.altitude)
            .map(|value| AltitudeBand {
                lower: Some(value),
                upper: Some(value),
            });
        for hazard in hazards.values() {
            let decision = rule.evaluate(RouteHazardInput {
                route,
                hazard,
                evaluated_at,
                route_altitude_band: altitude.as_ref(),
                progress: None,
            })?;
            if let Some(candidate) = candidate_from_route_hazard(route, hazard, decision) {
                store
                    .create_from_candidate(&candidate, evaluated_at)
                    .await?;
            }
        }
    }
    Ok(())
}

async fn persist_simulation_batch(
    database: &PgPool,
    batch: &NormalizedEventBatch,
) -> Result<(), sqlx::Error> {
    if batch.envelope.provider != "simulation" {
        return Ok(());
    }
    let mut transaction = database.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO operators (id, code, display_name)
        VALUES ($1, $2, 'Simulation operator')
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(batch.envelope.operator_id.as_uuid())
    .bind(format!(
        "SIM-{}",
        &batch.envelope.operator_id.as_uuid().simple().to_string()[..8]
    ))
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO provider_envelopes (
            id, operator_id, schema_version, provider, feed, provider_record_id,
            event_time, received_at, processed_at, raw_payload_sha256, raw_payload
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(batch.envelope.id.as_uuid())
    .bind(batch.envelope.operator_id.as_uuid())
    .bind(i16::try_from(batch.envelope.schema_version.get()).unwrap_or(i16::MAX))
    .bind(&batch.envelope.provider)
    .bind(&batch.envelope.feed)
    .bind(&batch.envelope.provider_record_id)
    .bind(batch.envelope.event_time)
    .bind(batch.envelope.received_at)
    .bind(batch.envelope.processed_at)
    .bind(&batch.envelope.raw_payload_sha256)
    .bind(&batch.envelope.raw_payload)
    .execute(&mut *transaction)
    .await?;

    for event in &batch.events {
        match event {
            CanonicalEvent::Flight(value) => insert_flight(&mut transaction, value).await?,
            CanonicalEvent::AircraftPosition(value) => {
                insert_position(&mut transaction, value).await?
            }
            CanonicalEvent::PlannedRoute(value) => insert_route(&mut transaction, value).await?,
            CanonicalEvent::WeatherHazard(value) => insert_hazard(&mut transaction, value).await?,
            _ => {}
        }
    }
    transaction.commit().await?;
    Ok(())
}

async fn insert_flight(
    transaction: &mut Transaction<'_, Postgres>,
    value: &Flight,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO flights (
            id, operator_id, source_envelope_id, schema_version, event_time, received_at,
            processed_at, callsign, aircraft_registration, origin_airport_code,
            destination_airport_code, scheduled_departure_at, scheduled_arrival_at, status
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        ON CONFLICT (operator_id, id) DO UPDATE SET
            source_envelope_id=EXCLUDED.source_envelope_id, event_time=EXCLUDED.event_time,
            received_at=EXCLUDED.received_at, processed_at=EXCLUDED.processed_at,
            status=EXCLUDED.status
        "#,
    )
    .bind(value.id.as_uuid())
    .bind(value.operator_id.as_uuid())
    .bind(value.source.envelope_id.as_uuid())
    .bind(i16::try_from(value.schema_version.get()).unwrap_or(i16::MAX))
    .bind(value.times.event_time)
    .bind(value.times.received_at)
    .bind(value.times.processed_at)
    .bind(&value.callsign)
    .bind(&value.aircraft_registration)
    .bind(&value.origin_airport_code)
    .bind(&value.destination_airport_code)
    .bind(value.scheduled_departure_at)
    .bind(value.scheduled_arrival_at)
    .bind(flight_status(value.status))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_position(
    transaction: &mut Transaction<'_, Postgres>,
    value: &AircraftPosition,
) -> Result<(), sqlx::Error> {
    let point = json!({"type":"Point","coordinates":value.point.as_geojson_position()});
    sqlx::query(
        r#"
        INSERT INTO aircraft_positions (
            id,operator_id,flight_id,source_envelope_id,schema_version,event_time,received_at,
            processed_at,position,altitude_value,altitude_unit,altitude_reference,
            heading_true_degrees,ground_speed_value,ground_speed_unit,source_quality
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,ST_SetSRID(ST_GeomFromGeoJSON($9),4326),$10,$11,$12,$13,$14,$15,$16)
        ON CONFLICT (operator_id,id) DO NOTHING
        "#,
    )
    .bind(value.id.as_uuid()).bind(value.operator_id.as_uuid()).bind(value.flight_id.as_uuid())
    .bind(value.source.envelope_id.as_uuid())
    .bind(i16::try_from(value.schema_version.get()).unwrap_or(i16::MAX))
    .bind(value.times.event_time).bind(value.times.received_at).bind(value.times.processed_at)
    .bind(point.to_string()).bind(value.altitude.map(|v| v.value))
    .bind(value.altitude.map(|v| altitude_unit(v.unit)))
    .bind(value.altitude.map(|v| altitude_reference(v.reference)))
    .bind(value.heading_true_degrees.map(Into::<f64>::into))
    .bind(value.ground_speed.map(|v| v.value))
    .bind(value.ground_speed.map(|v| speed_unit(v.unit))).bind(source_quality(value.quality))
    .execute(&mut **transaction).await?;
    Ok(())
}

async fn insert_route(
    transaction: &mut Transaction<'_, Postgres>,
    value: &PlannedRoute,
) -> Result<(), sqlx::Error> {
    let path = json!({"type":"LineString","coordinates":value.path.coordinates.iter().map(|p| p.as_geojson_position()).collect::<Vec<_>>()});
    sqlx::query(
        r#"
        INSERT INTO planned_routes (
            id,operator_id,flight_id,source_envelope_id,schema_version,event_time,received_at,
            processed_at,route_version,effective_from,effective_to,path
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,ST_SetSRID(ST_GeomFromGeoJSON($12),4326))
        ON CONFLICT (operator_id,id) DO NOTHING
        "#,
    )
    .bind(value.id.as_uuid())
    .bind(value.operator_id.as_uuid())
    .bind(value.flight_id.as_uuid())
    .bind(value.source.envelope_id.as_uuid())
    .bind(i16::try_from(value.schema_version.get()).unwrap_or(i16::MAX))
    .bind(value.times.event_time)
    .bind(value.times.received_at)
    .bind(value.times.processed_at)
    .bind(i32::try_from(value.route_version).unwrap_or(i32::MAX))
    .bind(value.effective_from)
    .bind(value.effective_to)
    .bind(path.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_hazard(
    transaction: &mut Transaction<'_, Postgres>,
    value: &WeatherHazard,
) -> Result<(), sqlx::Error> {
    let footprint = json!({"type":"Polygon","coordinates":[value.footprint.exterior.iter().map(|p| p.as_geojson_position()).collect::<Vec<_>>() ]});
    let lower = value.altitude_band.and_then(|band| band.lower);
    let upper = value.altitude_band.and_then(|band| band.upper);
    sqlx::query(
        r#"
        INSERT INTO weather_hazards (
            id,operator_id,source_envelope_id,schema_version,event_time,received_at,processed_at,
            external_series_id,revision,supersedes_id,status,issued_at,provider_received_at,
            hazard_type,severity,valid_from,valid_to,altitude_lower_value,altitude_lower_unit,
            altitude_lower_reference,altitude_upper_value,altitude_upper_unit,
            altitude_upper_reference,footprint
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,ST_SetSRID(ST_GeomFromGeoJSON($24),4326))
        ON CONFLICT (operator_id,id) DO NOTHING
        "#,
    )
    .bind(value.id.as_uuid()).bind(value.operator_id.as_uuid())
    .bind(value.source.envelope_id.as_uuid())
    .bind(i16::try_from(value.schema_version.get()).unwrap_or(i16::MAX))
    .bind(value.times.event_time).bind(value.times.received_at).bind(value.times.processed_at)
    .bind(&value.external_series_id).bind(i32::try_from(value.revision).unwrap_or(i32::MAX))
    .bind(value.supersedes_id.map(|id| id.as_uuid())).bind(hazard_status(value.status))
    .bind(value.issued_at).bind(value.provider_received_at).bind(&value.hazard_type)
    .bind(hazard_severity(value.severity)).bind(value.valid_from).bind(value.valid_to)
    .bind(lower.map(|v|v.value)).bind(lower.map(|v|altitude_unit(v.unit)))
    .bind(lower.map(|v|altitude_reference(v.reference))).bind(upper.map(|v|v.value))
    .bind(upper.map(|v|altitude_unit(v.unit))).bind(upper.map(|v|altitude_reference(v.reference)))
    .bind(footprint.to_string()).execute(&mut **transaction).await?;
    Ok(())
}

fn flight_status(value: FlightStatus) -> &'static str {
    match value {
        FlightStatus::Scheduled => "scheduled",
        FlightStatus::Active => "active",
        FlightStatus::Diverted => "diverted",
        FlightStatus::Landed => "landed",
        FlightStatus::Cancelled => "cancelled",
        FlightStatus::Unknown => "unknown",
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
fn altitude_unit(value: AltitudeUnit) -> &'static str {
    match value {
        AltitudeUnit::Feet => "feet",
        AltitudeUnit::Meters => "meters",
    }
}
fn altitude_reference(value: AltitudeReference) -> &'static str {
    match value {
        AltitudeReference::MeanSeaLevel => "mean_sea_level",
        AltitudeReference::AboveGroundLevel => "above_ground_level",
        AltitudeReference::FlightLevel => "flight_level",
        AltitudeReference::Ellipsoid => "ellipsoid",
    }
}
fn speed_unit(value: SpeedUnit) -> &'static str {
    match value {
        SpeedUnit::Knots => "knots",
        SpeedUnit::KilometersPerHour => "kilometers_per_hour",
    }
}
fn source_quality(value: SourceQuality) -> &'static str {
    match value {
        SourceQuality::Observed => "observed",
        SourceQuality::Fused => "fused",
        SourceQuality::Estimated => "estimated",
        SourceQuality::Unknown => "unknown",
    }
}
