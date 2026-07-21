use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::{
    AirportObservation, AirportObservationId, Altitude, AltitudeBand, AltitudeReference,
    AltitudeUnit, EventTimes, FlightCategory, GeoPoint, GeoPolygon, HazardSeverity, HeadingDegrees,
    OperatorId, ProviderEnvelope, ProviderEnvelopeId, SchemaVersion, SourceAttribution, Speed,
    SpeedUnit, WeatherHazardStatus,
};

use super::{NoaaFeed, NoaaPayload};

const NOAA_NAMESPACE: Uuid = Uuid::from_u128(0x56956acc_3ff4_5fc3_9b15_09f1167c8d42);
const PROVIDER: &str = "noaa-awc";

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedNoaaRecord {
    pub envelope: ProviderEnvelope,
    pub fact: Result<NoaaFactDraft, NormalizeFailure>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoaaFactDraft {
    Metar(AirportObservation),
    AirSigmet(SigmetDraft),
}

impl NoaaFactDraft {
    pub fn event_time(&self) -> DateTime<Utc> {
        match self {
            Self::Metar(value) => value.times.event_time,
            Self::AirSigmet(value) => value.issued_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SigmetDraft {
    pub operator_id: OperatorId,
    pub schema_version: SchemaVersion,
    pub source: SourceAttribution,
    pub times: EventTimes,
    pub external_series_id: String,
    pub status: WeatherHazardStatus,
    pub issued_at: DateTime<Utc>,
    pub provider_received_at: Option<DateTime<Utc>>,
    pub hazard_type: String,
    pub severity: HazardSeverity,
    pub valid_from: DateTime<Utc>,
    pub valid_to: DateTime<Utc>,
    pub altitude_band: Option<AltitudeBand>,
    pub footprint: Option<GeoPolygon>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {detail}")]
pub struct NormalizeFailure {
    pub code: &'static str,
    pub detail: String,
}

pub fn prepare_records(
    payload: NoaaPayload,
    operator_id: OperatorId,
    received_at: DateTime<Utc>,
) -> Vec<PreparedNoaaRecord> {
    let Some(value) = payload.value else {
        return Vec::new();
    };
    let items = match value {
        Value::Array(items) => items,
        other => {
            return vec![prepare_record(
                payload.feed,
                operator_id,
                received_at,
                other,
            )];
        }
    };
    items
        .into_iter()
        .map(|item| prepare_record(payload.feed, operator_id, received_at, item))
        .collect()
}

fn prepare_record(
    feed: NoaaFeed,
    operator_id: OperatorId,
    received_at: DateTime<Utc>,
    raw_payload: Value,
) -> PreparedNoaaRecord {
    let raw_bytes = serde_json::to_vec(&raw_payload).expect("JSON values always serialize");
    let raw_payload_sha256 = format!("{:x}", Sha256::digest(&raw_bytes));
    let provider_record_id = provider_record_id(feed, &raw_payload);
    let identity = format!(
        "{}:{}:{}:{}:{}",
        operator_id.as_uuid(),
        PROVIDER,
        feed.as_str(),
        provider_record_id.as_deref().unwrap_or("unidentified"),
        raw_payload_sha256
    );
    let envelope_id =
        ProviderEnvelopeId::from_uuid(Uuid::new_v5(&NOAA_NAMESPACE, identity.as_bytes()));
    let source = SourceAttribution {
        envelope_id,
        provider: PROVIDER.into(),
        feed: feed.as_str().into(),
        provider_record_id: provider_record_id.clone(),
    };
    let fact = match feed {
        NoaaFeed::Metar => normalize_metar(raw_payload.clone(), operator_id, source, received_at)
            .map(NoaaFactDraft::Metar),
        NoaaFeed::AirSigmet => {
            normalize_sigmet(raw_payload.clone(), operator_id, source, received_at)
                .map(NoaaFactDraft::AirSigmet)
        }
    };
    let event_time = fact.as_ref().ok().map(NoaaFactDraft::event_time);
    PreparedNoaaRecord {
        envelope: ProviderEnvelope {
            id: envelope_id,
            operator_id,
            schema_version: SchemaVersion::V1,
            provider: PROVIDER.into(),
            feed: feed.as_str().into(),
            provider_record_id,
            event_time,
            received_at,
            processed_at: fact.as_ref().ok().map(|_| received_at),
            raw_payload_sha256,
            raw_payload,
        },
        fact,
    }
}

fn normalize_metar(
    value: Value,
    operator_id: OperatorId,
    source: SourceAttribution,
    received_at: DateTime<Utc>,
) -> Result<AirportObservation, NormalizeFailure> {
    let record: MetarRecord = serde_json::from_value(value).map_err(invalid_shape)?;
    require_nonempty("icaoId", &record.icao_id)?;
    require_nonempty("rawOb", &record.raw_ob)?;
    let report_time = parse_rfc3339("reportTime", &record.report_time)?;
    let provider_received_at = parse_rfc3339("receiptTime", &record.receipt_time)?;
    let processed_at = received_at;
    let times = EventTimes::new(report_time, received_at, processed_at).map_err(|error| {
        NormalizeFailure {
            code: "invalid_times",
            detail: error.to_string(),
        }
    })?;
    let wind_direction_true_degrees = parse_wind_direction(record.wind_direction)?;
    let (visibility_statute_miles, visibility_greater_than) = parse_visibility(record.visibility)?;
    let ceiling = ceiling_from(&record.clouds, record.vertical_visibility);
    Ok(AirportObservation {
        id: AirportObservationId::from_uuid(Uuid::new_v5(
            &source.envelope_id.as_uuid(),
            b"airport-observation",
        )),
        operator_id,
        schema_version: SchemaVersion::V1,
        source,
        times,
        station_code: record.icao_id,
        report_type: record.metar_type.unwrap_or_else(|| "METAR".into()),
        raw_text: record.raw_ob,
        provider_received_at,
        point: GeoPoint::new(record.lon, record.lat).map_err(|error| NormalizeFailure {
            code: "invalid_geometry",
            detail: error.to_string(),
        })?,
        wind_direction_true_degrees,
        wind_speed: record.wind_speed.map(knots),
        wind_gust: record.wind_gust.map(knots),
        visibility_statute_miles,
        visibility_greater_than,
        ceiling,
        flight_category: parse_flight_category(record.flight_category.as_deref()),
    })
}

fn normalize_sigmet(
    value: Value,
    operator_id: OperatorId,
    source: SourceAttribution,
    received_at: DateTime<Utc>,
) -> Result<SigmetDraft, NormalizeFailure> {
    let record: AirSigmetRecord = serde_json::from_value(value).map_err(invalid_shape)?;
    require_nonempty("icaoId", &record.icao_id)?;
    require_nonempty("seriesId", &record.series_id)?;
    require_nonempty("hazard", &record.hazard)?;
    require_nonempty("rawAirSigmet", &record.raw_air_sigmet)?;
    let issued_at = parse_rfc3339("creationTime", &record.creation_time)?;
    let valid_from = parse_timestamp("validTimeFrom", &record.valid_time_from)?;
    let valid_to = parse_timestamp("validTimeTo", &record.valid_time_to)?;
    if valid_to < valid_from {
        return Err(NormalizeFailure {
            code: "invalid_validity",
            detail: "validTimeTo precedes validTimeFrom".into(),
        });
    }
    let provider_received_at = record
        .receipt_time
        .as_deref()
        .map(|value| parse_rfc3339("receiptTime", value))
        .transpose()?;
    let status = if record.raw_air_sigmet.contains("CNL SIGMET") {
        WeatherHazardStatus::Cancelled
    } else {
        WeatherHazardStatus::Active
    };
    let points = record
        .coordinates
        .into_iter()
        .map(|coordinate| {
            GeoPoint::new(coordinate.lon, coordinate.lat).map_err(|error| NormalizeFailure {
                code: "invalid_geometry",
                detail: error.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let footprint = if points.is_empty() && status == WeatherHazardStatus::Cancelled {
        None
    } else {
        Some(closed_polygon(points)?)
    };
    let external_series_id = format!(
        "{}:{}:{}",
        record.icao_id,
        record.series_id,
        valid_from.format("%Y-%m-%d")
    );
    let times =
        EventTimes::new(issued_at, received_at, received_at).map_err(|error| NormalizeFailure {
            code: "invalid_times",
            detail: error.to_string(),
        })?;
    Ok(SigmetDraft {
        operator_id,
        schema_version: SchemaVersion::V1,
        source,
        times,
        external_series_id,
        status,
        issued_at,
        provider_received_at,
        hazard_type: record.hazard.clone(),
        severity: sigmet_severity(&record.hazard, &record.raw_air_sigmet),
        valid_from,
        valid_to,
        altitude_band: altitude_band(
            record.altitude_low_1.or(record.altitude_low_2),
            record.altitude_high_1.or(record.altitude_high_2),
        ),
        footprint,
    })
}

fn provider_record_id(feed: NoaaFeed, value: &Value) -> Option<String> {
    match feed {
        NoaaFeed::Metar => Some(format!(
            "{}:{}",
            value.get("icaoId")?.as_str()?,
            value.get("reportTime")?.as_str()?
        )),
        NoaaFeed::AirSigmet => {
            let valid = value.get("validTimeFrom")?;
            Some(format!(
                "{}:{}:{}",
                value.get("icaoId")?.as_str()?,
                value.get("seriesId")?.as_str()?,
                timestamp_identity(valid)
            ))
        }
    }
}

fn timestamp_identity(value: &Value) -> String {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .unwrap_or_else(|| "invalid-time".into())
}

#[derive(Debug, Deserialize)]
struct MetarRecord {
    #[serde(rename = "icaoId")]
    icao_id: String,
    #[serde(rename = "receiptTime")]
    receipt_time: String,
    #[serde(rename = "reportTime")]
    report_time: String,
    #[serde(rename = "rawOb")]
    raw_ob: String,
    lat: f64,
    lon: f64,
    #[serde(rename = "metarType")]
    metar_type: Option<String>,
    #[serde(rename = "wdir")]
    wind_direction: Option<Value>,
    #[serde(rename = "wspd")]
    wind_speed: Option<f64>,
    #[serde(rename = "wgst")]
    wind_gust: Option<f64>,
    #[serde(rename = "visib")]
    visibility: Option<Value>,
    #[serde(default)]
    clouds: Vec<CloudLayer>,
    #[serde(rename = "vertVis")]
    vertical_visibility: Option<i32>,
    #[serde(rename = "fltCat")]
    flight_category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CloudLayer {
    cover: String,
    base: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct AirSigmetRecord {
    #[serde(rename = "icaoId")]
    icao_id: String,
    #[serde(rename = "seriesId")]
    series_id: String,
    #[serde(rename = "receiptTime")]
    receipt_time: Option<String>,
    #[serde(rename = "creationTime")]
    creation_time: String,
    #[serde(rename = "validTimeFrom")]
    valid_time_from: Value,
    #[serde(rename = "validTimeTo")]
    valid_time_to: Value,
    hazard: String,
    #[serde(rename = "altitudeHi1")]
    altitude_high_1: Option<i32>,
    #[serde(rename = "altitudeHi2")]
    altitude_high_2: Option<i32>,
    #[serde(rename = "altitudeLo1", alias = "altitudeLow1")]
    altitude_low_1: Option<i32>,
    #[serde(rename = "altitudeLo2", alias = "altitudeLow2")]
    altitude_low_2: Option<i32>,
    #[serde(rename = "rawAirSigmet")]
    raw_air_sigmet: String,
    #[serde(rename = "coords", default)]
    coordinates: Vec<Coordinate>,
}

#[derive(Debug, Deserialize)]
struct Coordinate {
    lat: f64,
    lon: f64,
}

fn invalid_shape(error: serde_json::Error) -> NormalizeFailure {
    NormalizeFailure {
        code: "invalid_shape",
        detail: error.to_string(),
    }
}

fn require_nonempty(field: &str, value: &str) -> Result<(), NormalizeFailure> {
    if value.trim().is_empty() {
        Err(NormalizeFailure {
            code: "missing_field",
            detail: format!("{field} must not be empty"),
        })
    } else {
        Ok(())
    }
}

fn parse_rfc3339(field: &str, value: &str) -> Result<DateTime<Utc>, NormalizeFailure> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| NormalizeFailure {
            code: "invalid_time",
            detail: format!("{field}: {error}"),
        })
}

fn parse_timestamp(field: &str, value: &Value) -> Result<DateTime<Utc>, NormalizeFailure> {
    if let Some(value) = value.as_str() {
        return parse_rfc3339(field, value);
    }
    if let Some(value) = value.as_i64() {
        return Utc
            .timestamp_opt(value, 0)
            .single()
            .ok_or_else(|| NormalizeFailure {
                code: "invalid_time",
                detail: format!("{field}: UNIX timestamp is out of range"),
            });
    }
    Err(NormalizeFailure {
        code: "invalid_time",
        detail: format!("{field}: expected RFC 3339 string or UNIX timestamp"),
    })
}

fn parse_wind_direction(value: Option<Value>) -> Result<Option<HeadingDegrees>, NormalizeFailure> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.as_str().is_some_and(|value| value == "VRB") {
        return Ok(None);
    }
    let degrees = value.as_f64().ok_or_else(|| NormalizeFailure {
        code: "invalid_wind",
        detail: "wdir must be degrees or VRB".into(),
    })?;
    degrees
        .try_into()
        .map(Some)
        .map_err(|error: crate::domain::MeasurementError| NormalizeFailure {
            code: "invalid_wind",
            detail: error.to_string(),
        })
}

fn parse_visibility(value: Option<Value>) -> Result<(Option<f64>, bool), NormalizeFailure> {
    let Some(value) = value else {
        return Ok((None, false));
    };
    if let Some(number) = value.as_f64() {
        return Ok((Some(number), false));
    }
    let text = value.as_str().ok_or_else(|| NormalizeFailure {
        code: "invalid_visibility",
        detail: "visib must be numeric or a numeric string".into(),
    })?;
    let greater_than = text.ends_with('+');
    let number = text
        .trim_end_matches('+')
        .parse::<f64>()
        .map_err(|error| NormalizeFailure {
            code: "invalid_visibility",
            detail: error.to_string(),
        })?;
    Ok((Some(number), greater_than))
}

fn ceiling_from(clouds: &[CloudLayer], vertical_visibility: Option<i32>) -> Option<Altitude> {
    let cloud_ceiling = clouds
        .iter()
        .filter(|layer| matches!(layer.cover.as_str(), "BKN" | "OVC" | "VV"))
        .filter_map(|layer| layer.base)
        .min();
    cloud_ceiling.or(vertical_visibility).map(|value| Altitude {
        value,
        unit: AltitudeUnit::Feet,
        reference: AltitudeReference::AboveGroundLevel,
    })
}

fn parse_flight_category(value: Option<&str>) -> FlightCategory {
    match value {
        Some("VFR") => FlightCategory::Visual,
        Some("MVFR") => FlightCategory::MarginalVisual,
        Some("IFR") => FlightCategory::Instrument,
        Some("LIFR") => FlightCategory::LowInstrument,
        _ => FlightCategory::Unknown,
    }
}

fn closed_polygon(mut points: Vec<GeoPoint>) -> Result<GeoPolygon, NormalizeFailure> {
    if points.len() < 3 {
        return Err(NormalizeFailure {
            code: "invalid_geometry",
            detail: "SIGMET polygon needs at least three distinct points".into(),
        });
    }
    if points.first() != points.last() {
        points.push(points[0]);
    }
    if points.len() < 4 {
        return Err(NormalizeFailure {
            code: "invalid_geometry",
            detail: "SIGMET polygon ring needs at least four positions".into(),
        });
    }
    Ok(GeoPolygon { exterior: points })
}

fn altitude_band(lower: Option<i32>, upper: Option<i32>) -> Option<AltitudeBand> {
    if lower.is_none() && upper.is_none() {
        return None;
    }
    Some(AltitudeBand {
        lower: lower.map(flight_level_feet),
        upper: upper.map(flight_level_feet),
    })
}

fn flight_level_feet(value: i32) -> Altitude {
    Altitude {
        value,
        unit: AltitudeUnit::Feet,
        reference: AltitudeReference::FlightLevel,
    }
}

fn knots(value: f64) -> Speed {
    Speed {
        value,
        unit: SpeedUnit::Knots,
    }
}

fn sigmet_severity(hazard: &str, raw: &str) -> HazardSeverity {
    if raw.contains("SEV ") {
        HazardSeverity::Severe
    } else if hazard.eq_ignore_ascii_case("CONVECTIVE") || raw.contains("SIGMET") {
        HazardSeverity::Significant
    } else {
        HazardSeverity::Unknown
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn operator_id() -> OperatorId {
        OperatorId::from_uuid(Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
    }

    fn received_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 21, 5, 3, 0).unwrap()
    }

    fn payload(feed: NoaaFeed, source: &str) -> NoaaPayload {
        NoaaPayload {
            feed,
            value: Some(serde_json::from_str(source).unwrap()),
        }
    }

    #[test]
    fn normal_metar_preserves_timing_units_and_operational_fields() {
        let records = prepare_records(
            payload(
                NoaaFeed::Metar,
                include_str!("../../../../../fixtures/noaa/metar-normal.json"),
            ),
            operator_id(),
            received_at(),
        );
        let NoaaFactDraft::Metar(observation) = records[0].fact.as_ref().unwrap() else {
            panic!("expected METAR");
        };
        assert_eq!(observation.station_code, "KSFO");
        assert_eq!(
            observation.point.as_geojson_position(),
            [-122.3656, 37.6196]
        );
        assert_eq!(
            observation.provider_received_at.to_rfc3339(),
            "2026-07-21T05:00:46.314+00:00"
        );
        assert_eq!(observation.times.received_at, received_at());
        assert_eq!(observation.visibility_statute_miles, Some(10.0));
        assert!(observation.visibility_greater_than);
        assert_eq!(observation.ceiling.unwrap().value, 2_000);
        assert_eq!(observation.flight_category, FlightCategory::Visual);
    }

    #[test]
    fn malformed_metar_is_retained_as_an_unprocessed_envelope() {
        let records = prepare_records(
            payload(
                NoaaFeed::Metar,
                include_str!("../../../../../fixtures/noaa/metar-malformed.json"),
            ),
            operator_id(),
            received_at(),
        );
        assert!(records[0].fact.is_err());
        assert!(records[0].envelope.processed_at.is_none());
        assert_eq!(
            records[0].envelope.raw_payload["rawOb"],
            "METAR KSFO MALFORMED"
        );
    }

    #[test]
    fn sigmet_normalization_preserves_issue_validity_altitude_and_geometry() {
        let records = prepare_records(
            payload(
                NoaaFeed::AirSigmet,
                include_str!("../../../../../fixtures/noaa/airsigmet-normal.json"),
            ),
            operator_id(),
            received_at(),
        );
        let NoaaFactDraft::AirSigmet(sigmet) = records[0].fact.as_ref().unwrap() else {
            panic!("expected SIGMET");
        };
        assert_eq!(sigmet.external_series_id, "KKCI:21W:2026-07-21");
        assert_eq!(sigmet.status, WeatherHazardStatus::Active);
        assert_eq!(sigmet.issued_at.to_rfc3339(), "2026-07-21T04:55:00+00:00");
        assert_eq!(sigmet.valid_to.to_rfc3339(), "2026-07-21T06:55:00+00:00");
        assert_eq!(sigmet.altitude_band.unwrap().upper.unwrap().value, 41_000);
        let footprint = sigmet.footprint.as_ref().unwrap();
        assert_eq!(footprint.exterior.first(), footprint.exterior.last());
    }

    #[test]
    fn duplicate_and_amended_payloads_have_stable_record_identity_but_distinct_envelopes() {
        let normal = prepare_records(
            payload(
                NoaaFeed::AirSigmet,
                include_str!("../../../../../fixtures/noaa/airsigmet-normal.json"),
            ),
            operator_id(),
            received_at(),
        );
        let duplicate = prepare_records(
            payload(
                NoaaFeed::AirSigmet,
                include_str!("../../../../../fixtures/noaa/airsigmet-normal.json"),
            ),
            operator_id(),
            received_at(),
        );
        let amended = prepare_records(
            payload(
                NoaaFeed::AirSigmet,
                include_str!("../../../../../fixtures/noaa/airsigmet-amended.json"),
            ),
            operator_id(),
            received_at(),
        );
        assert_eq!(normal[0].envelope.id, duplicate[0].envelope.id);
        assert_eq!(
            normal[0].envelope.provider_record_id,
            amended[0].envelope.provider_record_id
        );
        assert_ne!(normal[0].envelope.id, amended[0].envelope.id);
    }

    #[test]
    fn cancellation_is_an_explicit_hazard_status() {
        let records = prepare_records(
            payload(
                NoaaFeed::AirSigmet,
                include_str!("../../../../../fixtures/noaa/airsigmet-cancelled.json"),
            ),
            operator_id(),
            received_at(),
        );
        let NoaaFactDraft::AirSigmet(sigmet) = records[0].fact.as_ref().unwrap() else {
            panic!("expected SIGMET");
        };
        assert_eq!(sigmet.status, WeatherHazardStatus::Cancelled);
        assert!(sigmet.footprint.is_none());
    }
}
