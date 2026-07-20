use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::{
    AircraftPositionId, AlertActionId, AlertId, FlightId, GeoLineString, GeoPoint, GeoPolygon,
    OperatorId, PlannedRouteId, ProviderEnvelopeId, SourceHealthId, WeatherHazardId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SchemaVersion(u16);

impl SchemaVersion {
    pub const V1: Self = Self(1);

    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("schema version must be greater than zero")]
pub struct InvalidSchemaVersion;

impl TryFrom<u16> for SchemaVersion {
    type Error = InvalidSchemaVersion;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value == 0 {
            Err(InvalidSchemaVersion)
        } else {
            Ok(Self(value))
        }
    }
}

impl From<SchemaVersion> for u16 {
    fn from(value: SchemaVersion) -> Self {
        value.get()
    }
}

/// Times for a normalized fact. All values are UTC instants serialized as
/// RFC 3339. Event time belongs to the source event, received time is when this
/// system accepted it, and processed time is when normalization completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTimes {
    pub event_time: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("processed time cannot precede received time")]
pub struct TimeValidationError;

impl EventTimes {
    pub fn new(
        event_time: DateTime<Utc>,
        received_at: DateTime<Utc>,
        processed_at: DateTime<Utc>,
    ) -> Result<Self, TimeValidationError> {
        if processed_at < received_at {
            return Err(TimeValidationError);
        }

        Ok(Self {
            event_time,
            received_at,
            processed_at,
        })
    }
}

/// The immutable raw provider boundary. `event_time` and `processed_at` are
/// nullable because a provider record may omit event time or fail before it can
/// be normalized. Raw JSON never appears on normalized entity types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderEnvelope {
    pub id: ProviderEnvelopeId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub provider: String,
    pub feed: String,
    pub provider_record_id: Option<String>,
    pub event_time: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub raw_payload_sha256: String,
    pub raw_payload: Value,
}

/// Provenance carried by every normalized external fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAttribution {
    pub envelope_id: ProviderEnvelopeId,
    pub provider: String,
    pub feed: String,
    pub provider_record_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltitudeUnit {
    Feet,
    Meters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltitudeReference {
    MeanSeaLevel,
    AboveGroundLevel,
    FlightLevel,
    Ellipsoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Altitude {
    pub value: i32,
    pub unit: AltitudeUnit,
    pub reference: AltitudeReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltitudeBand {
    pub lower: Option<Altitude>,
    pub upper: Option<Altitude>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedUnit {
    Knots,
    KilometersPerHour,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Speed {
    pub value: f64,
    pub unit: SpeedUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct HeadingDegrees(f64);

#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum MeasurementError {
    #[error("heading must be finite and in the half-open range [0, 360), got {0}")]
    InvalidHeading(f64),
}

impl TryFrom<f64> for HeadingDegrees {
    type Error = MeasurementError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && (0.0..360.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MeasurementError::InvalidHeading(value))
        }
    }
}

impl From<HeadingDegrees> for f64 {
    fn from(value: HeadingDegrees) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceQuality {
    Observed,
    Fused,
    Estimated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlightStatus {
    Scheduled,
    Active,
    Diverted,
    Landed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flight {
    pub id: FlightId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub source: SourceAttribution,
    pub times: EventTimes,
    pub callsign: Option<String>,
    pub aircraft_registration: Option<String>,
    pub origin_airport_code: Option<String>,
    pub destination_airport_code: Option<String>,
    pub scheduled_departure_at: Option<DateTime<Utc>>,
    pub scheduled_arrival_at: Option<DateTime<Utc>>,
    pub status: FlightStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AircraftPosition {
    pub id: AircraftPositionId,
    pub operator_id: OperatorId,
    pub flight_id: FlightId,
    pub schema_version: SchemaVersion,
    pub source: SourceAttribution,
    pub times: EventTimes,
    pub point: GeoPoint,
    pub altitude: Option<Altitude>,
    pub heading_true_degrees: Option<HeadingDegrees>,
    pub ground_speed: Option<Speed>,
    pub quality: SourceQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoute {
    pub id: PlannedRouteId,
    pub operator_id: OperatorId,
    pub flight_id: FlightId,
    pub schema_version: SchemaVersion,
    pub source: SourceAttribution,
    pub times: EventTimes,
    pub route_version: u32,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub path: GeoLineString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardSeverity {
    Advisory,
    Significant,
    Severe,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherHazard {
    pub id: WeatherHazardId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub source: SourceAttribution,
    pub times: EventTimes,
    pub hazard_type: String,
    pub severity: HazardSeverity,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    pub altitude_band: Option<AltitudeBand>,
    pub footprint: GeoPolygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Information,
    Advisory,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLifecycle {
    Open,
    Acknowledged,
    Dismissed,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    pub id: AlertId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub times: EventTimes,
    pub flight_id: Option<FlightId>,
    pub hazard_id: Option<WeatherHazardId>,
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub lifecycle: AlertLifecycle,
    pub rule_id: String,
    pub rule_version: u32,
    pub dedupe_key: String,
    pub evidence_envelope_ids: Vec<ProviderEnvelopeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertActionKind {
    Acknowledge,
    Dismiss,
    Comment,
    Resolve,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertAction {
    pub id: AlertActionId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub alert_id: AlertId,
    pub action: AlertActionKind,
    pub actor_id: String,
    pub occurred_at: DateTime<Utc>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceHealthState {
    Healthy,
    Degraded,
    Stale,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceHealth {
    pub id: SourceHealthId,
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub provider: String,
    pub feed: String,
    pub state: SourceHealthState,
    pub observed_at: DateTime<Utc>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub delay_seconds: Option<u64>,
    pub stale_after_seconds: u64,
    pub last_error_code: Option<String>,
}

/// Tagged normalized contract used by replay and ingestion boundaries. Provider
/// envelopes are intentionally excluded: they are inputs, not canonical facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data", rename_all = "snake_case")]
pub enum CanonicalEvent {
    Flight(Flight),
    AircraftPosition(AircraftPosition),
    PlannedRoute(PlannedRoute),
    WeatherHazard(WeatherHazard),
    Alert(Alert),
    AlertAction(AlertAction),
    SourceHealth(SourceHealth),
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    fn uuid(value: &str) -> uuid::Uuid {
        Uuid::parse_str(value).unwrap()
    }

    fn event_times() -> EventTimes {
        EventTimes::new(
            Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 1).unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 2).unwrap(),
        )
        .unwrap()
    }

    fn source() -> SourceAttribution {
        SourceAttribution {
            envelope_id: ProviderEnvelopeId::from_uuid(uuid(
                "00000000-0000-0000-0000-000000000010",
            )),
            provider: "simulation".to_owned(),
            feed: "scenario-alpha".to_owned(),
            provider_record_id: Some("position-42".to_owned()),
        }
    }

    #[test]
    fn representative_position_serialization_is_versioned_and_explicit() {
        let position = CanonicalEvent::AircraftPosition(AircraftPosition {
            id: AircraftPositionId::from_uuid(uuid("00000000-0000-0000-0000-000000000020")),
            operator_id: OperatorId::from_uuid(uuid("00000000-0000-0000-0000-000000000001")),
            flight_id: FlightId::from_uuid(uuid("00000000-0000-0000-0000-000000000030")),
            schema_version: SchemaVersion::V1,
            source: source(),
            times: event_times(),
            point: GeoPoint::new(-122.3656, 37.6196).unwrap(),
            altitude: Some(Altitude {
                value: 12_000,
                unit: AltitudeUnit::Feet,
                reference: AltitudeReference::MeanSeaLevel,
            }),
            heading_true_degrees: Some(330.0.try_into().unwrap()),
            ground_speed: Some(Speed {
                value: 420.0,
                unit: SpeedUnit::Knots,
            }),
            quality: SourceQuality::Observed,
        });

        let value = serde_json::to_value(&position).unwrap();
        assert_eq!(value["event_type"], "aircraft_position");
        assert_eq!(value["data"]["schema_version"], 1);
        assert_eq!(value["data"]["point"]["longitude_degrees"], -122.3656);
        assert_eq!(value["data"]["point"]["latitude_degrees"], 37.6196);
        assert_eq!(value["data"]["altitude"]["unit"], "feet");
        assert_eq!(value["data"]["ground_speed"]["unit"], "knots");
        assert_eq!(value["data"]["times"]["event_time"], "2026-07-20T20:00:00Z");
        assert!(value["data"].get("raw_payload").is_none());

        let round_trip: CanonicalEvent = serde_json::from_value(value).unwrap();
        assert_eq!(round_trip, position);
    }

    #[test]
    fn raw_provider_payload_remains_on_the_envelope_boundary() {
        let envelope = ProviderEnvelope {
            id: ProviderEnvelopeId::from_uuid(uuid("00000000-0000-0000-0000-000000000010")),
            operator_id: OperatorId::from_uuid(uuid("00000000-0000-0000-0000-000000000001")),
            schema_version: SchemaVersion::V1,
            provider: "simulation".to_owned(),
            feed: "scenario-alpha".to_owned(),
            provider_record_id: Some("position-42".to_owned()),
            event_time: Some(Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 0).unwrap()),
            received_at: Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 1).unwrap(),
            processed_at: Some(Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 2).unwrap()),
            raw_payload_sha256: "a".repeat(64),
            raw_payload: json!({"providerSpecific": true}),
        };

        let value = serde_json::to_value(envelope).unwrap();
        assert_eq!(value["raw_payload"]["providerSpecific"], true);
        assert_eq!(value["schema_version"], 1);
    }

    #[test]
    fn processed_time_cannot_precede_received_time() {
        let received = Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 2).unwrap();
        let processed = Utc.with_ymd_and_hms(2026, 7, 20, 20, 0, 1).unwrap();

        assert!(EventTimes::new(received, received, processed).is_err());
    }

    #[test]
    fn zero_schema_version_is_rejected_during_deserialization() {
        assert!(serde_json::from_value::<SchemaVersion>(json!(0)).is_err());
    }

    #[test]
    fn heading_rejects_wrapped_and_non_finite_values() {
        assert!(HeadingDegrees::try_from(360.0).is_err());
        assert!(HeadingDegrees::try_from(-0.1).is_err());
        assert!(HeadingDegrees::try_from(f64::INFINITY).is_err());
    }
}
