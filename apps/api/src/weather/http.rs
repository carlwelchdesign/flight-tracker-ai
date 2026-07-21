use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::auth::{AuthContext, Permission, require};

const MAX_HAZARDS: i64 = 500;
const MAX_OBSERVATIONS: i64 = 200;

#[derive(Clone)]
struct WeatherHttpState {
    database: PgPool,
}

#[derive(Debug, Serialize)]
struct WeatherPage<T> {
    data: Vec<T>,
    generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SourceView {
    envelope_id: Uuid,
    provider: String,
    feed: String,
    provider_record_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct EventTimesView {
    event_time: DateTime<Utc>,
    received_at: DateTime<Utc>,
    processed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PointView {
    longitude_degrees: f64,
    latitude_degrees: f64,
}

#[derive(Debug, Serialize)]
struct PolygonView {
    exterior: Vec<PointView>,
}

#[derive(Debug, Serialize)]
struct AltitudeView {
    value: i32,
    unit: String,
    reference: String,
}

#[derive(Debug, Serialize)]
struct AltitudeBandView {
    lower: Option<AltitudeView>,
    upper: Option<AltitudeView>,
}

#[derive(Debug, Serialize)]
struct HazardView {
    id: Uuid,
    operator_id: Uuid,
    schema_version: i16,
    source: SourceView,
    times: EventTimesView,
    external_series_id: String,
    revision: i32,
    supersedes_id: Option<Uuid>,
    status: String,
    issued_at: DateTime<Utc>,
    provider_received_at: Option<DateTime<Utc>>,
    hazard_type: String,
    severity: String,
    valid_from: DateTime<Utc>,
    valid_to: DateTime<Utc>,
    altitude_band: Option<AltitudeBandView>,
    footprint: PolygonView,
}

#[derive(Debug, FromRow)]
struct HazardRow {
    id: Uuid,
    operator_id: Uuid,
    source_envelope_id: Uuid,
    schema_version: i16,
    event_time: DateTime<Utc>,
    received_at: DateTime<Utc>,
    processed_at: DateTime<Utc>,
    external_series_id: String,
    revision: i32,
    supersedes_id: Option<Uuid>,
    status: String,
    issued_at: DateTime<Utc>,
    provider_received_at: Option<DateTime<Utc>>,
    hazard_type: String,
    severity: String,
    valid_from: DateTime<Utc>,
    valid_to: DateTime<Utc>,
    altitude_lower_value: Option<i32>,
    altitude_lower_unit: Option<String>,
    altitude_lower_reference: Option<String>,
    altitude_upper_value: Option<i32>,
    altitude_upper_unit: Option<String>,
    altitude_upper_reference: Option<String>,
    footprint_geojson: String,
    provider: String,
    feed: String,
    provider_record_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ObservationView {
    id: Uuid,
    operator_id: Uuid,
    schema_version: i16,
    source: SourceView,
    times: EventTimesView,
    station_code: String,
    report_type: String,
    raw_text: String,
    provider_received_at: DateTime<Utc>,
    point: PointView,
    wind_direction_true_degrees: Option<f64>,
    wind_speed: Option<SpeedView>,
    wind_gust: Option<SpeedView>,
    visibility_statute_miles: Option<f64>,
    visibility_greater_than: bool,
    ceiling: Option<AltitudeView>,
    flight_category: String,
}

#[derive(Debug, Serialize)]
struct SpeedView {
    value: f64,
    unit: &'static str,
}

#[derive(Debug, FromRow)]
struct ObservationRow {
    id: Uuid,
    operator_id: Uuid,
    source_envelope_id: Uuid,
    schema_version: i16,
    event_time: DateTime<Utc>,
    received_at: DateTime<Utc>,
    processed_at: DateTime<Utc>,
    station_code: String,
    report_type: String,
    raw_text: String,
    provider_received_at: DateTime<Utc>,
    longitude_degrees: f64,
    latitude_degrees: f64,
    wind_direction_true_degrees: Option<f64>,
    wind_speed_knots: Option<f64>,
    wind_gust_knots: Option<f64>,
    visibility_statute_miles: Option<f64>,
    visibility_greater_than: bool,
    ceiling_feet_agl: Option<i32>,
    flight_category: String,
    provider: String,
    feed: String,
    provider_record_id: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct SourceRecordView {
    id: Uuid,
    provider: String,
    feed: String,
    provider_record_id: Option<String>,
    event_time: Option<DateTime<Utc>>,
    received_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
    raw_payload_sha256: String,
    raw_payload: Value,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

enum WeatherApiError {
    InvalidSourceId,
    SourceNotFound,
    Unavailable,
    InvalidGeometry,
}

impl IntoResponse for WeatherApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidSourceId => (
                StatusCode::BAD_REQUEST,
                "invalid_source_id",
                "Source record ID must be a UUID",
            ),
            Self::SourceNotFound => (
                StatusCode::NOT_FOUND,
                "source_record_not_found",
                "NOAA source record was not found",
            ),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "weather_unavailable",
                "Weather evidence is temporarily unavailable",
            ),
            Self::InvalidGeometry => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "invalid_weather_geometry",
                "Stored weather geometry could not be read",
            ),
        };
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

pub fn weather_router(database: PgPool) -> Router {
    Router::new()
        .route("/api/hazards", get(list_hazards))
        .route("/api/airport-observations", get(list_observations))
        .route("/api/source-records/{envelope_id}", get(source_record))
        .with_state(WeatherHttpState { database })
}

async fn list_hazards(
    State(state): State<WeatherHttpState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<WeatherPage<HazardView>>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let rows = sqlx::query_as::<_, HazardRow>(
        r#"
        WITH latest AS (
            SELECT wh.*,
                   ROW_NUMBER() OVER (
                       PARTITION BY wh.operator_id, wh.external_series_id
                       ORDER BY wh.revision DESC, wh.issued_at DESC, wh.id DESC
                   ) AS series_rank
            FROM weather_hazards wh
            WHERE wh.operator_id = $1
        )
        SELECT latest.id, latest.operator_id, latest.source_envelope_id,
               latest.schema_version, latest.event_time, latest.received_at,
               latest.processed_at, latest.external_series_id, latest.revision,
               latest.supersedes_id, latest.status, latest.issued_at,
               latest.provider_received_at, latest.hazard_type, latest.severity,
               latest.valid_from, latest.valid_to, latest.altitude_lower_value,
               latest.altitude_lower_unit, latest.altitude_lower_reference,
               latest.altitude_upper_value, latest.altitude_upper_unit,
               latest.altitude_upper_reference,
               ST_AsGeoJSON(latest.footprint)::text AS footprint_geojson,
               envelope.provider, envelope.feed, envelope.provider_record_id
        FROM latest
        JOIN provider_envelopes envelope
          ON envelope.operator_id = latest.operator_id
         AND envelope.id = latest.source_envelope_id
        WHERE latest.series_rank = 1
          AND latest.valid_to >= NOW() - INTERVAL '6 hours'
        ORDER BY latest.valid_to DESC, latest.issued_at DESC
        LIMIT $2
        "#,
    )
    .bind(context.operator_id.as_uuid())
    .bind(MAX_HAZARDS)
    .fetch_all(&state.database)
    .await
    .map_err(|error| {
        tracing::warn!(error = %error, "weather hazard read failed");
        WeatherApiError::Unavailable.into_response()
    })?;
    let data = rows
        .into_iter()
        .map(HazardView::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(IntoResponse::into_response)?;
    Ok(Json(WeatherPage {
        data,
        generated_at: Utc::now(),
    }))
}

async fn list_observations(
    State(state): State<WeatherHttpState>,
    Extension(context): Extension<AuthContext>,
) -> Result<Json<WeatherPage<ObservationView>>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let rows = sqlx::query_as::<_, ObservationRow>(
        r#"
        SELECT * FROM (
            SELECT DISTINCT ON (observation.operator_id, observation.station_code)
                   observation.id, observation.operator_id,
                   observation.source_envelope_id, observation.schema_version,
                   observation.event_time, observation.received_at,
                   observation.processed_at, observation.station_code,
                   observation.report_type, observation.raw_text,
                   observation.provider_received_at,
                   ST_X(observation.position) AS longitude_degrees,
                   ST_Y(observation.position) AS latitude_degrees,
                   observation.wind_direction_true_degrees,
                   observation.wind_speed_knots, observation.wind_gust_knots,
                   observation.visibility_statute_miles,
                   observation.visibility_greater_than,
                   observation.ceiling_feet_agl, observation.flight_category,
                   envelope.provider, envelope.feed, envelope.provider_record_id
            FROM airport_observations observation
            JOIN provider_envelopes envelope
              ON envelope.operator_id = observation.operator_id
             AND envelope.id = observation.source_envelope_id
            WHERE observation.operator_id = $1
              AND observation.event_time >= NOW() - INTERVAL '2 hours'
            ORDER BY observation.operator_id, observation.station_code,
                     observation.event_time DESC, observation.id DESC
        ) latest
        ORDER BY event_time DESC
        LIMIT $2
        "#,
    )
    .bind(context.operator_id.as_uuid())
    .bind(MAX_OBSERVATIONS)
    .fetch_all(&state.database)
    .await
    .map_err(|error| {
        tracing::warn!(error = %error, "airport observation read failed");
        WeatherApiError::Unavailable.into_response()
    })?;
    Ok(Json(WeatherPage {
        data: rows.into_iter().map(ObservationView::from).collect(),
        generated_at: Utc::now(),
    }))
}

async fn source_record(
    State(state): State<WeatherHttpState>,
    Extension(context): Extension<AuthContext>,
    Path(envelope_id): Path<String>,
) -> Result<Json<SourceRecordView>, Response> {
    require(&context, Permission::ReadOperations).map_err(IntoResponse::into_response)?;
    let envelope_id = Uuid::parse_str(&envelope_id)
        .map_err(|_| WeatherApiError::InvalidSourceId.into_response())?;
    let record = sqlx::query_as::<_, SourceRecordView>(
        r#"
        SELECT id, provider, feed, provider_record_id, event_time, received_at,
               processed_at, raw_payload_sha256, raw_payload
        FROM provider_envelopes
        WHERE id = $1 AND operator_id = $2 AND provider = 'noaa-awc'
          AND raw_payload_deleted_at IS NULL
        "#,
    )
    .bind(envelope_id)
    .bind(context.operator_id.as_uuid())
    .fetch_optional(&state.database)
    .await
    .map_err(|error| {
        tracing::warn!(error = %error, "NOAA source record read failed");
        WeatherApiError::Unavailable.into_response()
    })?
    .ok_or_else(|| WeatherApiError::SourceNotFound.into_response())?;
    Ok(Json(record))
}

impl TryFrom<HazardRow> for HazardView {
    type Error = WeatherApiError;

    fn try_from(row: HazardRow) -> Result<Self, Self::Error> {
        #[derive(serde::Deserialize)]
        struct GeoJsonPolygon {
            coordinates: Vec<Vec<[f64; 2]>>,
        }
        let polygon: GeoJsonPolygon = serde_json::from_str(&row.footprint_geojson)
            .map_err(|_| WeatherApiError::InvalidGeometry)?;
        let exterior = polygon
            .coordinates
            .into_iter()
            .next()
            .ok_or(WeatherApiError::InvalidGeometry)?
            .into_iter()
            .map(|[longitude_degrees, latitude_degrees]| PointView {
                longitude_degrees,
                latitude_degrees,
            })
            .collect();
        let lower = altitude(
            row.altitude_lower_value,
            row.altitude_lower_unit,
            row.altitude_lower_reference,
        );
        let upper = altitude(
            row.altitude_upper_value,
            row.altitude_upper_unit,
            row.altitude_upper_reference,
        );
        Ok(Self {
            id: row.id,
            operator_id: row.operator_id,
            schema_version: row.schema_version,
            source: SourceView {
                envelope_id: row.source_envelope_id,
                provider: row.provider,
                feed: row.feed,
                provider_record_id: row.provider_record_id,
            },
            times: EventTimesView {
                event_time: row.event_time,
                received_at: row.received_at,
                processed_at: row.processed_at,
            },
            external_series_id: row.external_series_id,
            revision: row.revision,
            supersedes_id: row.supersedes_id,
            status: row.status,
            issued_at: row.issued_at,
            provider_received_at: row.provider_received_at,
            hazard_type: row.hazard_type,
            severity: row.severity,
            valid_from: row.valid_from,
            valid_to: row.valid_to,
            altitude_band: (lower.is_some() || upper.is_some())
                .then_some(AltitudeBandView { lower, upper }),
            footprint: PolygonView { exterior },
        })
    }
}

impl From<ObservationRow> for ObservationView {
    fn from(row: ObservationRow) -> Self {
        Self {
            id: row.id,
            operator_id: row.operator_id,
            schema_version: row.schema_version,
            source: SourceView {
                envelope_id: row.source_envelope_id,
                provider: row.provider,
                feed: row.feed,
                provider_record_id: row.provider_record_id,
            },
            times: EventTimesView {
                event_time: row.event_time,
                received_at: row.received_at,
                processed_at: row.processed_at,
            },
            station_code: row.station_code,
            report_type: row.report_type,
            raw_text: row.raw_text,
            provider_received_at: row.provider_received_at,
            point: PointView {
                longitude_degrees: row.longitude_degrees,
                latitude_degrees: row.latitude_degrees,
            },
            wind_direction_true_degrees: row.wind_direction_true_degrees,
            wind_speed: row.wind_speed_knots.map(|value| SpeedView {
                value,
                unit: "knots",
            }),
            wind_gust: row.wind_gust_knots.map(|value| SpeedView {
                value,
                unit: "knots",
            }),
            visibility_statute_miles: row.visibility_statute_miles,
            visibility_greater_than: row.visibility_greater_than,
            ceiling: row.ceiling_feet_agl.map(|value| AltitudeView {
                value,
                unit: "feet".into(),
                reference: "above_ground_level".into(),
            }),
            flight_category: row.flight_category,
        }
    }
}

fn altitude(
    value: Option<i32>,
    unit: Option<String>,
    reference: Option<String>,
) -> Option<AltitudeView> {
    Some(AltitudeView {
        value: value?,
        unit: unit?,
        reference: reference?,
    })
}
