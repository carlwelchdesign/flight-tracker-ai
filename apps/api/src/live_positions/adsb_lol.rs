use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use rand::RngExt;
use reqwest::{StatusCode, Url, header};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AircraftPosition, AircraftPositionId, Altitude, AltitudeReference, AltitudeUnit,
        CanonicalEvent, EventTimes, Flight, FlightId, FlightStatus, GeoPoint, HeadingDegrees,
        OperatorId, ProviderEnvelope, ProviderEnvelopeId, SchemaVersion, SourceAttribution,
        SourceQuality, Speed, SpeedUnit,
    },
    ingestion::NormalizedEventBatch,
};

use super::status::{LivePositionProvider, LivePositionRegion, PositionCoverage};

const MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_POSITION_AGE_SECONDS: f64 = 300.0;
const ID_NAMESPACE: Uuid = Uuid::from_u128(0xd6254fe2_ecc1_5d9a_aac0_073def280ee7);
const FEED: &str = "point";

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(2),
        }
    }
}

impl RetryPolicy {
    fn delay_with_jitter(&self, retry_number: u32, jitter_fraction: f64) -> Duration {
        let exponent = retry_number.saturating_sub(1).min(31);
        let cap = self
            .base_delay
            .saturating_mul(1_u32 << exponent)
            .min(self.max_delay);
        cap.mul_f64(jitter_fraction.clamp(0.0, 1.0))
    }
}

#[derive(Debug, Clone)]
pub struct AdsbLolClientConfig {
    pub provider: LivePositionProvider,
    pub base_url: Url,
    pub user_agent: String,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub retry: RetryPolicy,
    pub minimum_request_interval: Option<Duration>,
}

#[derive(Debug, Error)]
pub enum AdsbLolError {
    #[error("live-position provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("live-position provider returned HTTP {status}")]
    Http { status: StatusCode },
    #[error("live-position provider response exceeded the one-megabyte safety limit")]
    ResponseTooLarge,
    #[error("live-position provider returned malformed JSON: {0}")]
    MalformedJson(serde_json::Error),
    #[error("live-position provider payload is invalid: {0}")]
    InvalidPayload(&'static str),
    #[error("live-position provider client configuration is invalid: {0}")]
    Configuration(String),
}

impl AdsbLolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Request(error) if error.is_timeout() => "timeout",
            Self::Request(_) => "request_failed",
            Self::Http { status } if *status == StatusCode::TOO_MANY_REQUESTS => "rate_limited",
            Self::Http { status } if status.is_server_error() => "provider_unavailable",
            Self::Http { .. } => "invalid_request",
            Self::ResponseTooLarge => "response_too_large",
            Self::MalformedJson(_) => "malformed_json",
            Self::InvalidPayload(_) => "invalid_payload",
            Self::Configuration(_) => "configuration",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AdsbLolPayload {
    pub provider: LivePositionProvider,
    pub value: Value,
    pub received_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AdsbLolClient {
    client: reqwest::Client,
    provider: LivePositionProvider,
    base_url: Url,
    retry: RetryPolicy,
    request_gate: Option<RequestGate>,
}

#[derive(Clone)]
struct RequestGate {
    next_allowed: Arc<tokio::sync::Mutex<tokio::time::Instant>>,
    minimum_interval: Duration,
}

impl RequestGate {
    fn new(minimum_interval: Duration) -> Self {
        Self {
            next_allowed: Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now())),
            minimum_interval,
        }
    }

    async fn wait(&self) {
        let mut next_allowed = self.next_allowed.lock().await;
        tokio::time::sleep_until(*next_allowed).await;
        *next_allowed = tokio::time::Instant::now() + self.minimum_interval;
    }
}

impl AdsbLolClient {
    pub fn new(config: AdsbLolClientConfig) -> Result<Self, AdsbLolError> {
        if config.user_agent.trim().is_empty() {
            return Err(AdsbLolError::Configuration(
                "user agent must identify the consumer".into(),
            ));
        }
        if !matches!(config.base_url.scheme(), "http" | "https") {
            return Err(AdsbLolError::Configuration(
                "base URL must use HTTP or HTTPS".into(),
            ));
        }
        if config.retry.max_attempts == 0 {
            return Err(AdsbLolError::Configuration(
                "max_attempts must be greater than zero".into(),
            ));
        }
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .user_agent(config.user_agent)
            .default_headers(headers)
            .build()?;
        Ok(Self {
            client,
            provider: config.provider,
            base_url: config.base_url,
            retry: config.retry,
            request_gate: config.minimum_request_interval.map(RequestGate::new),
        })
    }

    pub(crate) async fn fetch_point(
        &self,
        region: LivePositionRegion,
    ) -> Result<AdsbLolPayload, AdsbLolError> {
        let path = format!(
            "v2/point/{}/{}/{}",
            region.latitude_degrees, region.longitude_degrees, region.radius_nautical_miles
        );
        let url = self
            .base_url
            .join(&path)
            .map_err(|error| AdsbLolError::Configuration(error.to_string()))?;
        let mut attempt = 1;
        loop {
            if let Some(gate) = &self.request_gate {
                gate.wait().await;
            }
            let result = self.client.get(url.clone()).send().await;
            match result {
                Ok(response) if response.status().is_success() => {
                    return read_payload(response, self.provider).await;
                }
                Ok(response) => {
                    let status = response.status();
                    if attempt >= self.retry.max_attempts || !is_retryable_status(status) {
                        return Err(AdsbLolError::Http { status });
                    }
                }
                Err(error) => {
                    let retryable = error.is_timeout() || error.is_connect();
                    if attempt >= self.retry.max_attempts || !retryable {
                        return Err(AdsbLolError::Request(error));
                    }
                }
            }
            let jitter = rand::rng().random_range(0.0..=1.0);
            tokio::time::sleep(self.retry.delay_with_jitter(attempt, jitter)).await;
            attempt += 1;
        }
    }
}

async fn read_payload(
    mut response: reqwest::Response,
    provider: LivePositionProvider,
) -> Result<AdsbLolPayload, AdsbLolError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(AdsbLolError::ResponseTooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(AdsbLolError::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice(&bytes).map_err(AdsbLolError::MalformedJson)?;
    Ok(AdsbLolPayload {
        provider,
        value,
        received_at: Utc::now(),
    })
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || matches!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
}

#[derive(Debug)]
pub(crate) struct NormalizedAdsbLolSnapshot {
    pub batches: Vec<NormalizedEventBatch>,
    pub newest_position_at: Option<DateTime<Utc>>,
    pub coverage: PositionCoverage,
}

#[derive(Debug, Deserialize)]
struct AdsbLolResponse {
    now: i64,
    ac: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct AdsbLolAircraft {
    hex: String,
    #[serde(default)]
    flight: Option<String>,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    alt_baro: Option<AltitudeValue>,
    #[serde(default)]
    alt_geom: Option<f64>,
    #[serde(default)]
    gs: Option<f64>,
    #[serde(default)]
    track: Option<f64>,
    #[serde(default)]
    seen_pos: Option<f64>,
    #[serde(rename = "type", default)]
    source_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AltitudeValue {
    Number(f64),
    Text(String),
}

struct Candidate {
    hex: String,
    callsign: Option<String>,
    point: GeoPoint,
    altitude: Option<Altitude>,
    ground_speed: Option<Speed>,
    heading: Option<HeadingDegrees>,
    quality: SourceQuality,
    seen_pos_seconds: f64,
    event_time: DateTime<Utc>,
    raw: Value,
}

pub(crate) fn normalize_snapshot(
    payload: AdsbLolPayload,
    operator_id: OperatorId,
    processed_at: DateTime<Utc>,
    stale_after: Duration,
) -> Result<NormalizedAdsbLolSnapshot, AdsbLolError> {
    if processed_at < payload.received_at {
        return Err(AdsbLolError::InvalidPayload(
            "processed time precedes received time",
        ));
    }
    let provider = payload.provider;
    let response: AdsbLolResponse =
        serde_json::from_value(payload.value).map_err(AdsbLolError::MalformedJson)?;
    let response_time =
        Utc.timestamp_millis_opt(response.now)
            .single()
            .ok_or(AdsbLolError::InvalidPayload(
                "now is not a valid UNIX millisecond timestamp",
            ))?;
    let mut rejected = 0;
    let mut by_hex = HashMap::<String, Candidate>::new();

    for raw in response.ac {
        let aircraft: AdsbLolAircraft = match serde_json::from_value(raw.clone()) {
            Ok(value) => value,
            Err(_) => {
                rejected += 1;
                continue;
            }
        };
        let Some(hex) = normalize_hex(&aircraft.hex) else {
            rejected += 1;
            continue;
        };
        let (Some(latitude), Some(longitude), Some(seen_pos_seconds)) =
            (aircraft.lat, aircraft.lon, aircraft.seen_pos)
        else {
            rejected += 1;
            continue;
        };
        if !seen_pos_seconds.is_finite()
            || !(0.0..=MAX_POSITION_AGE_SECONDS).contains(&seen_pos_seconds)
        {
            rejected += 1;
            continue;
        }
        let point = match GeoPoint::new(longitude, latitude) {
            Ok(value) => value,
            Err(_) => {
                rejected += 1;
                continue;
            }
        };
        let age_millis = (seen_pos_seconds * 1_000.0).round() as i64;
        let event_time = response_time
            .checked_sub_signed(TimeDelta::milliseconds(age_millis))
            .ok_or(AdsbLolError::InvalidPayload(
                "position age overflows event time",
            ))?;
        let candidate = Candidate {
            hex: hex.clone(),
            callsign: normalize_callsign(aircraft.flight),
            point,
            altitude: normalize_altitude(aircraft.alt_geom, aircraft.alt_baro),
            ground_speed: normalize_speed(aircraft.gs),
            heading: aircraft.track.and_then(|value| value.try_into().ok()),
            quality: source_quality(aircraft.source_type.as_deref()),
            seen_pos_seconds,
            event_time,
            raw,
        };
        match by_hex.get(&hex) {
            Some(existing) if existing.seen_pos_seconds <= seen_pos_seconds => {
                rejected += 1;
            }
            Some(_) => {
                rejected += 1;
                by_hex.insert(hex, candidate);
            }
            _ => {
                by_hex.insert(hex, candidate);
            }
        }
    }

    let mut candidates = by_hex.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.hex.cmp(&right.hex));
    let mut batches = Vec::with_capacity(candidates.len());
    let mut newest_position_at = None;
    let mut fresh = 0;
    let mut stale = 0;
    let mut missing_callsign = 0;
    for candidate in candidates {
        if candidate.seen_pos_seconds <= stale_after.as_secs_f64() {
            fresh += 1;
        } else {
            stale += 1;
        }
        if candidate.callsign.is_none() {
            missing_callsign += 1;
        }
        newest_position_at = Some(
            newest_position_at.map_or(candidate.event_time, |current: DateTime<Utc>| {
                current.max(candidate.event_time)
            }),
        );
        batches.push(to_batch(
            candidate,
            operator_id,
            provider,
            payload.received_at,
            processed_at,
        )?);
    }

    Ok(NormalizedAdsbLolSnapshot {
        coverage: PositionCoverage {
            aircraft_count: batches.len(),
            fresh_position_count: fresh,
            stale_position_count: stale,
            rejected_record_count: rejected,
            missing_callsign_count: missing_callsign,
        },
        batches,
        newest_position_at,
    })
}

fn to_batch(
    candidate: Candidate,
    operator_id: OperatorId,
    provider: LivePositionProvider,
    received_at: DateTime<Utc>,
    processed_at: DateTime<Utc>,
) -> Result<NormalizedEventBatch, AdsbLolError> {
    let raw_bytes = serde_json::to_vec(&candidate.raw).map_err(AdsbLolError::MalformedJson)?;
    let raw_hash = format!("{:x}", Sha256::digest(&raw_bytes));
    let flight_identity = format!("{}:{}:live-position", operator_id.as_uuid(), candidate.hex);
    let flight_id = FlightId::from_uuid(Uuid::new_v5(&ID_NAMESPACE, flight_identity.as_bytes()));
    let envelope_identity = format!(
        "{}:{}:{}:{}:{}",
        operator_id.as_uuid(),
        candidate.hex,
        candidate.event_time.timestamp_millis(),
        provider.id(),
        raw_hash
    );
    let envelope_id =
        ProviderEnvelopeId::from_uuid(Uuid::new_v5(&ID_NAMESPACE, envelope_identity.as_bytes()));
    let position_id = AircraftPositionId::from_uuid(Uuid::new_v5(
        &ID_NAMESPACE,
        format!("{}:position", envelope_id.as_uuid()).as_bytes(),
    ));
    let source = SourceAttribution {
        envelope_id,
        provider: provider.id().into(),
        feed: FEED.into(),
        provider_record_id: Some(candidate.hex.clone()),
    };
    let times = EventTimes::new(candidate.event_time, received_at, processed_at)
        .map_err(|_| AdsbLolError::InvalidPayload("invalid normalization timestamps"))?;
    let envelope = ProviderEnvelope {
        id: envelope_id,
        operator_id,
        schema_version: SchemaVersion::V1,
        provider: provider.id().into(),
        feed: FEED.into(),
        provider_record_id: Some(candidate.hex),
        event_time: Some(candidate.event_time),
        received_at,
        processed_at: Some(processed_at),
        raw_payload_sha256: raw_hash,
        raw_payload: candidate.raw,
    };
    let flight = Flight {
        id: flight_id,
        operator_id,
        schema_version: SchemaVersion::V1,
        source: source.clone(),
        times: times.clone(),
        callsign: candidate.callsign,
        aircraft_registration: None,
        origin_airport_code: None,
        destination_airport_code: None,
        scheduled_departure_at: None,
        scheduled_arrival_at: None,
        status: FlightStatus::Unknown,
    };
    let position = AircraftPosition {
        id: position_id,
        operator_id,
        flight_id,
        schema_version: SchemaVersion::V1,
        source,
        times,
        point: candidate.point,
        altitude: candidate.altitude,
        heading_true_degrees: candidate.heading,
        ground_speed: candidate.ground_speed,
        quality: candidate.quality,
    };
    Ok(NormalizedEventBatch {
        envelope,
        events: vec![
            CanonicalEvent::Flight(flight),
            CanonicalEvent::AircraftPosition(position),
        ],
    })
}

fn normalize_hex(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let digits = value.strip_prefix('~').unwrap_or(&value);
    (digits.len() == 6
        && digits
            .chars()
            .all(|character| character.is_ascii_hexdigit()))
    .then_some(value)
}

fn normalize_callsign(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_altitude(
    geometric: Option<f64>,
    barometric: Option<AltitudeValue>,
) -> Option<Altitude> {
    if let Some(value) = finite_i32(geometric) {
        return Some(Altitude {
            value,
            unit: AltitudeUnit::Feet,
            reference: AltitudeReference::Ellipsoid,
        });
    }
    match barometric {
        Some(AltitudeValue::Number(value)) => finite_i32(Some(value)).map(|value| Altitude {
            value,
            unit: AltitudeUnit::Feet,
            reference: AltitudeReference::MeanSeaLevel,
        }),
        Some(AltitudeValue::Text(value)) if value.eq_ignore_ascii_case("ground") => None,
        _ => None,
    }
}

fn finite_i32(value: Option<f64>) -> Option<i32> {
    let value = value?;
    (value.is_finite() && value >= i32::MIN as f64 && value <= i32::MAX as f64)
        .then_some(value.round() as i32)
}

fn normalize_speed(value: Option<f64>) -> Option<Speed> {
    let value = value?;
    (value.is_finite() && value >= 0.0).then_some(Speed {
        value,
        unit: SpeedUnit::Knots,
    })
}

fn source_quality(value: Option<&str>) -> SourceQuality {
    match value {
        Some("adsb_icao" | "adsb_icao_nt" | "adsb_other" | "adsr_icao" | "adsr_other" | "adsc") => {
            SourceQuality::Observed
        }
        Some("mlat" | "tisb_icao" | "tisb_other" | "tisb_trackfile") => SourceQuality::Fused,
        Some("other") => SourceQuality::Unknown,
        _ => SourceQuality::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::{
        Json, Router,
        body::Body,
        extract::State,
        http::HeaderMap,
        response::{IntoResponse, Response},
        routing::get,
    };
    use chrono::TimeZone;
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;
    use crate::fleet::FleetStore;

    fn received_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 1).unwrap()
    }

    fn synthetic_payload(records: Vec<Value>) -> AdsbLolPayload {
        AdsbLolPayload {
            provider: LivePositionProvider::AdsbLol,
            value: json!({ "now": 1784654160000_i64, "ac": records }),
            received_at: received_at(),
        }
    }

    #[test]
    fn maps_only_position_and_identity_facts_without_inventing_route_or_status() {
        let snapshot = normalize_snapshot(
            synthetic_payload(vec![json!({
                "hex": "A1B2C3",
                "flight": " TEST42 ",
                "lat": 37.62,
                "lon": -122.38,
                "alt_baro": 12000,
                "alt_geom": 12100,
                "gs": 410.5,
                "track": 275.0,
                "seen_pos": 2.5,
                "type": "adsb_icao",
                "origin": "SFO",
                "destination": "JFK"
            })]),
            OperatorId::new(),
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(snapshot.coverage.aircraft_count, 1);
        assert_eq!(snapshot.coverage.fresh_position_count, 1);
        let events = &snapshot.batches[0].events;
        let CanonicalEvent::Flight(flight) = &events[0] else {
            panic!("expected flight");
        };
        assert_eq!(flight.callsign.as_deref(), Some("TEST42"));
        assert_eq!(flight.origin_airport_code, None);
        assert_eq!(flight.destination_airport_code, None);
        assert_eq!(flight.status, FlightStatus::Unknown);
        let CanonicalEvent::AircraftPosition(position) = &events[1] else {
            panic!("expected position");
        };
        assert_eq!(position.point.as_geojson_position(), [-122.38, 37.62]);
        assert_eq!(position.quality, SourceQuality::Observed);
        assert_eq!(
            position.altitude.unwrap().reference,
            AltitudeReference::Ellipsoid
        );
    }

    #[test]
    fn fallback_provider_identity_reaches_every_canonical_source() {
        let mut payload = synthetic_payload(vec![json!({
            "hex": "A1B2C3",
            "lat": 37.62,
            "lon": -122.38,
            "seen_pos": 2.5,
            "type": "adsb_icao"
        })]);
        payload.provider = LivePositionProvider::AirplanesLive;
        let snapshot = normalize_snapshot(
            payload,
            OperatorId::new(),
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        let batch = &snapshot.batches[0];
        assert_eq!(batch.envelope.provider, "airplanes.live");
        for event in &batch.events {
            match event {
                CanonicalEvent::Flight(value) => {
                    assert_eq!(value.source.provider, "airplanes.live")
                }
                CanonicalEvent::AircraftPosition(value) => {
                    assert_eq!(value.source.provider, "airplanes.live")
                }
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn duplicate_and_out_of_order_snapshots_cannot_replace_newer_state() {
        let operator = OperatorId::new();
        let store = FleetStore::new(32);
        let newer = normalize_snapshot(
            synthetic_payload(vec![json!({
                "hex": "a1b2c3", "flight": "TEST42", "lat": 37.62,
                "lon": -122.38, "seen_pos": 1.0, "type": "adsb_icao"
            })]),
            operator,
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        let duplicate = normalize_snapshot(
            synthetic_payload(vec![json!({
                "hex": "a1b2c3", "flight": "TEST42", "lat": 37.62,
                "lon": -122.38, "seen_pos": 1.0, "type": "adsb_icao"
            })]),
            operator,
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        let older = normalize_snapshot(
            AdsbLolPayload {
                provider: LivePositionProvider::AdsbLol,
                value: json!({
                    "now": 1784654100000_i64,
                    "ac": [{"hex": "a1b2c3", "lat": 40.0, "lon": -120.0,
                            "seen_pos": 1.0, "type": "adsb_icao"}]
                }),
                received_at: received_at(),
            },
            operator,
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();

        let first = store.apply(&newer.batches[0]).await.unwrap();
        let second = store.apply(&duplicate.batches[0]).await.unwrap();
        let third = store.apply(&older.batches[0]).await.unwrap();
        assert_eq!(first.accepted_events, 2);
        assert!(second.duplicate_batch);
        assert_eq!(third.accepted_events, 0);
        let flight_id = match &newer.batches[0].events[0] {
            CanonicalEvent::Flight(value) => value.id,
            _ => unreachable!(),
        };
        let detail = store.detail(operator, flight_id).await.unwrap();
        assert_eq!(
            detail.latest_position.unwrap().point.as_geojson_position(),
            [-122.38, 37.62]
        );
    }

    #[test]
    fn rejects_invalid_records_and_keeps_the_freshest_duplicate_identity() {
        let snapshot = normalize_snapshot(
            synthetic_payload(vec![
                json!({"hex": "a1b2c3", "lat": 37.0, "lon": -122.0, "seen_pos": 20.0}),
                json!({"hex": "A1B2C3", "lat": 38.0, "lon": -121.0, "seen_pos": 2.0}),
                json!({"hex": "bad", "lat": 0, "lon": 0, "seen_pos": 1.0}),
                json!({"hex": "abcdef", "lat": 95, "lon": 0, "seen_pos": 1.0}),
                json!({"hex": "fedcba", "lat": 1, "lon": 1, "seen_pos": 301.0}),
            ]),
            OperatorId::new(),
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(snapshot.coverage.aircraft_count, 1);
        assert_eq!(snapshot.coverage.rejected_record_count, 4);
        let CanonicalEvent::AircraftPosition(position) = &snapshot.batches[0].events[1] else {
            unreachable!()
        };
        assert_eq!(position.point.as_geojson_position(), [-121.0, 38.0]);
    }

    #[test]
    fn rejects_malformed_top_level_payload_without_publishing_partial_state() {
        let result = normalize_snapshot(
            AdsbLolPayload {
                provider: LivePositionProvider::AdsbLol,
                value: json!({
                    "now": "not-a-timestamp",
                    "ac": {"unexpected": "object"}
                }),
                received_at: received_at(),
            },
            OperatorId::new(),
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        );
        assert!(matches!(result, Err(AdsbLolError::MalformedJson(_))));
    }

    #[derive(Clone, Default)]
    struct TestState {
        attempts: Arc<AtomicUsize>,
        path: Arc<Mutex<Option<String>>>,
        user_agent: Arc<Mutex<Option<String>>>,
    }

    #[tokio::test]
    async fn regional_client_retries_rate_limit_with_jitter_policy_and_no_global_query() {
        async fn point(
            State(state): State<TestState>,
            headers: HeaderMap,
            request: axum::extract::Request,
        ) -> impl IntoResponse {
            *state.path.lock().unwrap() = Some(request.uri().path().to_owned());
            *state.user_agent.lock().unwrap() = headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .map(ToOwned::to_owned);
            if state.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                return StatusCode::TOO_MANY_REQUESTS.into_response();
            }
            Json(json!({ "now": 1784654160000_i64, "ac": [] })).into_response()
        }

        let state = TestState::default();
        let router = Router::new()
            .route("/v2/point/37.62/-122.38/25", get(point))
            .with_state(state.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = AdsbLolClient::new(AdsbLolClientConfig {
            provider: LivePositionProvider::AdsbLol,
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy {
                max_attempts: 2,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
            minimum_request_interval: None,
        })
        .unwrap();
        client
            .fetch_point(LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            })
            .await
            .unwrap();
        assert_eq!(state.attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            state.path.lock().unwrap().as_deref(),
            Some("/v2/point/37.62/-122.38/25")
        );
        assert_eq!(
            state.user_agent.lock().unwrap().as_deref(),
            Some("flight-tracker-ai-test/1.0")
        );
    }

    #[tokio::test]
    async fn request_timeout_is_bounded_and_classified_for_replay_fallback() {
        let router = Router::new().route(
            "/v2/point/37.62/-122.38/25",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Json(json!({ "now": 1784654160000_i64, "ac": [] }))
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = AdsbLolClient::new(AdsbLolClientConfig {
            provider: LivePositionProvider::AdsbLol,
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_millis(10),
            request_timeout: Duration::from_millis(5),
            retry: RetryPolicy {
                max_attempts: 1,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
            minimum_request_interval: None,
        })
        .unwrap();
        let error = client
            .fetch_point(LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            })
            .await
            .unwrap_err();
        assert_eq!(error.code(), "timeout");
    }

    #[tokio::test]
    async fn oversized_response_is_rejected_before_its_body_is_retained() {
        async fn oversized() -> Response {
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, MAX_RESPONSE_BYTES + 1)
                .body(Body::from(vec![b' '; MAX_RESPONSE_BYTES + 1]))
                .unwrap()
        }

        let router = Router::new().route("/v2/point/37.62/-122.38/25", get(oversized));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = AdsbLolClient::new(AdsbLolClientConfig {
            provider: LivePositionProvider::AdsbLol,
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy {
                max_attempts: 1,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
            minimum_request_interval: None,
        })
        .unwrap();

        let error = client
            .fetch_point(LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            })
            .await
            .unwrap_err();
        assert!(matches!(error, AdsbLolError::ResponseTooLarge));
        assert_eq!(error.code(), "response_too_large");
    }

    #[tokio::test]
    async fn cloned_clients_share_the_fallback_request_gate() {
        let router = Router::new().route(
            "/v2/point/37.62/-122.38/25",
            get(|| async { Json(json!({ "now": 1784654160000_i64, "ac": [] })) }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = AdsbLolClient::new(AdsbLolClientConfig {
            provider: LivePositionProvider::AirplanesLive,
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            user_agent: "flight-tracker-ai-test/1.0".into(),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            retry: RetryPolicy {
                max_attempts: 1,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
            },
            minimum_request_interval: Some(Duration::from_millis(50)),
        })
        .unwrap();
        let second_client = client.clone();
        let started = tokio::time::Instant::now();
        let (first, second) = tokio::join!(
            client.fetch_point(LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            }),
            second_client.fetch_point(LivePositionRegion {
                latitude_degrees: 37.62,
                longitude_degrees: -122.38,
                radius_nautical_miles: 25,
            })
        );

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert!(started.elapsed() >= Duration::from_millis(45));
    }

    #[test]
    fn stale_positions_remain_visible_but_are_counted_as_stale() {
        let snapshot = normalize_snapshot(
            synthetic_payload(vec![json!({
                "hex": "a1b2c3", "lat": 37.62, "lon": -122.38,
                "seen_pos": 45.0, "type": "mlat"
            })]),
            OperatorId::new(),
            Utc.with_ymd_and_hms(2026, 7, 21, 17, 16, 2).unwrap(),
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(snapshot.coverage.fresh_position_count, 0);
        assert_eq!(snapshot.coverage.stale_position_count, 1);
        let CanonicalEvent::AircraftPosition(position) = &snapshot.batches[0].events[1] else {
            unreachable!()
        };
        assert_eq!(position.quality, SourceQuality::Fused);
    }

    #[test]
    fn exponential_backoff_is_bounded_and_jittered() {
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(250),
        };
        assert_eq!(policy.delay_with_jitter(1, 0.0), Duration::ZERO);
        assert_eq!(policy.delay_with_jitter(1, 1.0), Duration::from_millis(100));
        assert_eq!(policy.delay_with_jitter(2, 1.0), Duration::from_millis(200));
        assert_eq!(policy.delay_with_jitter(3, 1.0), Duration::from_millis(250));
    }
}
