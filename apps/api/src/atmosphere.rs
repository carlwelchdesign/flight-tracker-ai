use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::live_positions::{LivePositionRegion, find_public_live_region_definition};

const MAX_RESPONSE_BYTES: usize = 262_144;
const GRID_OFFSETS: [f64; 4] = [-1.2, -0.4, 0.4, 1.2];
const PROVIDER_SOURCE_URL: &str = "https://open-meteo.com/";
const PROVIDER_LICENSE_URL: &str = "https://open-meteo.com/en/license";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindLevel {
    Surface,
    Hpa850,
    Hpa700,
    Hpa500,
    Hpa300,
}

impl WindLevel {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "surface" => Some(Self::Surface),
            "850" => Some(Self::Hpa850),
            "700" => Some(Self::Hpa700),
            "500" => Some(Self::Hpa500),
            "300" => Some(Self::Hpa300),
            _ => None,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Surface => "surface",
            Self::Hpa850 => "850",
            Self::Hpa700 => "700",
            Self::Hpa500 => "500",
            Self::Hpa300 => "300",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Surface => "Surface · 10 m AGL",
            Self::Hpa850 => "850 hPa · about 5,000 ft",
            Self::Hpa700 => "700 hPa · about 10,000 ft",
            Self::Hpa500 => "500 hPa · about 18,000 ft",
            Self::Hpa300 => "300 hPa · about 30,000 ft",
        }
    }

    fn pressure_hpa(self) -> Option<u16> {
        match self {
            Self::Surface => None,
            Self::Hpa850 => Some(850),
            Self::Hpa700 => Some(700),
            Self::Hpa500 => Some(500),
            Self::Hpa300 => Some(300),
        }
    }

    fn approximate_altitude_feet(self) -> u32 {
        match self {
            Self::Surface => 33,
            Self::Hpa850 => 4_900,
            Self::Hpa700 => 9_800,
            Self::Hpa500 => 18_400,
            Self::Hpa300 => 30_200,
        }
    }

    fn variables(self) -> (String, String) {
        match self.pressure_hpa() {
            None => ("wind_speed_10m".into(), "wind_direction_10m".into()),
            Some(level) => (
                format!("wind_speed_{level}hPa"),
                format!("wind_direction_{level}hPa"),
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AtmosphereQuery {
    #[serde(default = "default_region")]
    region: String,
    #[serde(default = "default_level")]
    level: String,
}

fn default_region() -> String {
    "sfo".into()
}

fn default_level() -> String {
    "surface".into()
}

#[derive(Debug, Clone, Serialize)]
pub struct WindFieldSnapshot {
    state: &'static str,
    retained: bool,
    region_code: &'static str,
    region_name: &'static str,
    level: WindLevelView,
    generated_at: DateTime<Utc>,
    forecast_time: DateTime<Utc>,
    last_success_at: DateTime<Utc>,
    last_error_code: Option<&'static str>,
    attribution: WindAttribution,
    samples: Vec<WindSample>,
}

#[derive(Debug, Clone, Serialize)]
struct WindLevelView {
    code: &'static str,
    label: &'static str,
    pressure_hpa: Option<u16>,
    approximate_altitude_feet: u32,
}

#[derive(Debug, Clone, Serialize)]
struct WindAttribution {
    provider: &'static str,
    model: &'static str,
    source_url: &'static str,
    license_url: &'static str,
    text: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindSample {
    latitude_degrees: f64,
    longitude_degrees: f64,
    speed_knots: f64,
    direction_from_degrees: f64,
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    region: String,
    level: WindLevel,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    accepted_at: DateTime<Utc>,
    snapshot: WindFieldSnapshot,
}

#[derive(Clone)]
pub struct AtmosphereService {
    client: reqwest::Client,
    base_url: Url,
    cache: Arc<Mutex<HashMap<CacheKey, CacheEntry>>>,
    refresh_after: Duration,
    retain_for: Duration,
}

#[derive(Debug, Error)]
pub enum AtmosphereError {
    #[error("atmospheric provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("atmospheric provider returned HTTP {0}")]
    Http(StatusCode),
    #[error("atmospheric provider response exceeded the safety limit")]
    ResponseTooLarge,
    #[error("atmospheric provider returned malformed JSON: {0}")]
    MalformedJson(serde_json::Error),
    #[error("atmospheric provider payload is invalid: {0}")]
    InvalidPayload(&'static str),
    #[error("atmospheric client configuration is invalid: {0}")]
    Configuration(String),
}

impl AtmosphereError {
    fn code(&self) -> &'static str {
        match self {
            Self::Request(error) if error.is_timeout() => "timeout",
            Self::Request(_) => "request_failed",
            Self::Http(StatusCode::TOO_MANY_REQUESTS) => "rate_limited",
            Self::Http(status) if status.is_server_error() => "provider_unavailable",
            Self::Http(_) => "invalid_request",
            Self::ResponseTooLarge => "response_too_large",
            Self::MalformedJson(_) => "malformed_json",
            Self::InvalidPayload(_) => "invalid_payload",
            Self::Configuration(_) => "configuration",
        }
    }
}

impl AtmosphereService {
    pub fn production() -> Result<Self, AtmosphereError> {
        Self::new(
            Url::parse("https://api.open-meteo.com/v1/gfs")
                .map_err(|error| AtmosphereError::Configuration(error.to_string()))?,
            Duration::from_secs(15 * 60),
            Duration::from_secs(2 * 60 * 60),
        )
    }

    pub fn new(
        base_url: Url,
        refresh_after: Duration,
        retain_for: Duration,
    ) -> Result<Self, AtmosphereError> {
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(AtmosphereError::Configuration(
                "base URL must use HTTP or HTTPS".into(),
            ));
        }
        if refresh_after.is_zero() || retain_for < refresh_after {
            return Err(AtmosphereError::Configuration(
                "cache windows must be positive and retain must cover refresh".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(10))
            .user_agent(
                "flight-tracker-ai/0.1 (+https://github.com/carlwelchdesign/flight-tracker-ai)",
            )
            .build()?;
        Ok(Self {
            client,
            base_url,
            cache: Arc::new(Mutex::new(HashMap::new())),
            refresh_after,
            retain_for,
        })
    }

    async fn snapshot(
        &self,
        region_code: &str,
        level: WindLevel,
    ) -> Result<WindFieldSnapshot, AtmosphereError> {
        let region = find_public_live_region_definition(region_code)
            .ok_or(AtmosphereError::InvalidPayload("unknown region"))?;
        let key = CacheKey {
            region: region.code.into(),
            level,
        };
        let now = Utc::now();
        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get(&key)
            && age(now, entry.accepted_at) < self.refresh_after
        {
            return Ok(entry.snapshot.clone());
        }

        match self
            .fetch(region.region, region.code, region.name, level)
            .await
        {
            Ok(snapshot) => {
                cache.insert(
                    key,
                    CacheEntry {
                        accepted_at: now,
                        snapshot: snapshot.clone(),
                    },
                );
                Ok(snapshot)
            }
            Err(error) => {
                if let Some(entry) = cache.get(&key)
                    && age(now, entry.accepted_at) < self.retain_for
                {
                    let mut retained = entry.snapshot.clone();
                    retained.state = "degraded";
                    retained.retained = true;
                    retained.generated_at = now;
                    retained.last_error_code = Some(error.code());
                    return Ok(retained);
                }
                Err(error)
            }
        }
    }

    async fn fetch(
        &self,
        region: LivePositionRegion,
        region_code: &'static str,
        region_name: &'static str,
        level: WindLevel,
    ) -> Result<WindFieldSnapshot, AtmosphereError> {
        let points = grid_points(region);
        let latitudes = join_coordinates(points.iter().map(|point| point.0));
        let longitudes = join_coordinates(points.iter().map(|point| point.1));
        let (speed_variable, direction_variable) = level.variables();
        let current = format!("{speed_variable},{direction_variable}");
        let response = self
            .client
            .get(self.base_url.clone())
            .query(&[
                ("latitude", latitudes.as_str()),
                ("longitude", longitudes.as_str()),
                ("current", current.as_str()),
                ("wind_speed_unit", "kn"),
                ("forecast_days", "1"),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AtmosphereError::Http(response.status()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(AtmosphereError::ResponseTooLarge);
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(AtmosphereError::ResponseTooLarge);
        }
        let value = serde_json::from_slice(&bytes).map_err(AtmosphereError::MalformedJson)?;
        parse_snapshot(
            value,
            region_code,
            region_name,
            level,
            &speed_variable,
            &direction_variable,
            Utc::now(),
        )
    }
}

pub fn public_atmosphere_router(service: AtmosphereService) -> Router {
    Router::new()
        .route("/api/public/atmosphere/wind", get(public_wind_field))
        .with_state(service)
}

async fn public_wind_field(
    State(service): State<AtmosphereService>,
    Query(query): Query<AtmosphereQuery>,
) -> Response {
    let region_code = query.region.to_ascii_lowercase();
    if find_public_live_region_definition(&region_code).is_none() {
        return public_error(
            StatusCode::NOT_FOUND,
            "atmosphere_region_not_found",
            "The requested atmospheric region is not available",
        );
    }
    let Some(level) = WindLevel::parse(&query.level.to_ascii_lowercase()) else {
        return public_error(
            StatusCode::NOT_FOUND,
            "atmosphere_level_not_found",
            "The requested atmospheric wind level is not available",
        );
    };
    match service.snapshot(&region_code, level).await {
        Ok(snapshot) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "no-store")],
            Json(snapshot),
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(error_code = error.code(), error = %error, "atmospheric wind unavailable");
            public_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "atmosphere_unavailable",
                "Atmospheric model wind is temporarily unavailable",
            )
        }
    }
}

fn public_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ErrorEnvelope {
            error: ErrorDetail { code, message },
        }),
    )
        .into_response()
}

fn parse_snapshot(
    value: Value,
    region_code: &'static str,
    region_name: &'static str,
    level: WindLevel,
    speed_variable: &str,
    direction_variable: &str,
    now: DateTime<Utc>,
) -> Result<WindFieldSnapshot, AtmosphereError> {
    let records = match value {
        Value::Array(records) => records,
        Value::Object(_) => vec![value],
        _ => {
            return Err(AtmosphereError::InvalidPayload(
                "root must be an object or array",
            ));
        }
    };
    if records.len() != GRID_OFFSETS.len() * GRID_OFFSETS.len() {
        return Err(AtmosphereError::InvalidPayload("unexpected grid size"));
    }
    let mut samples = Vec::with_capacity(records.len());
    let mut forecast_time: Option<DateTime<Utc>> = None;
    for record in records {
        let latitude = finite_number(&record, "latitude")?;
        let longitude = finite_number(&record, "longitude")?;
        if !(-90.0..=90.0).contains(&latitude) || !(-180.0..=180.0).contains(&longitude) {
            return Err(AtmosphereError::InvalidPayload("coordinate outside WGS84"));
        }
        let current = record
            .get("current")
            .and_then(Value::as_object)
            .ok_or(AtmosphereError::InvalidPayload("missing current values"))?;
        let time = current
            .get("time")
            .and_then(Value::as_str)
            .and_then(parse_model_time)
            .ok_or(AtmosphereError::InvalidPayload("invalid model time"))?;
        if forecast_time.is_some_and(|accepted| accepted != time) {
            return Err(AtmosphereError::InvalidPayload("mixed model times"));
        }
        forecast_time = Some(time);
        let speed = current
            .get(speed_variable)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=250.0).contains(value))
            .ok_or(AtmosphereError::InvalidPayload("invalid wind speed"))?;
        let direction = current
            .get(direction_variable)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=360.0).contains(value))
            .ok_or(AtmosphereError::InvalidPayload("invalid wind direction"))?;
        samples.push(WindSample {
            latitude_degrees: latitude,
            longitude_degrees: longitude,
            speed_knots: speed,
            direction_from_degrees: direction,
        });
    }
    Ok(WindFieldSnapshot {
        state: "current",
        retained: false,
        region_code,
        region_name,
        level: WindLevelView {
            code: level.code(),
            label: level.label(),
            pressure_hpa: level.pressure_hpa(),
            approximate_altitude_feet: level.approximate_altitude_feet(),
        },
        generated_at: now,
        forecast_time: forecast_time.expect("non-empty validated grid"),
        last_success_at: now,
        last_error_code: None,
        attribution: WindAttribution {
            provider: "Open-Meteo",
            model: "NOAA GFS / HRRR",
            source_url: PROVIDER_SOURCE_URL,
            license_url: PROVIDER_LICENSE_URL,
            text: "NOAA GFS/HRRR model data delivered by Open-Meteo",
        },
        samples,
    })
}

fn finite_number(value: &Value, key: &'static str) -> Result<f64, AtmosphereError> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or(AtmosphereError::InvalidPayload(key))
}

fn parse_model_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
                .ok()
                .map(|value| value.and_utc())
        })
}

fn grid_points(region: LivePositionRegion) -> Vec<(f64, f64)> {
    let longitude_scale = region.latitude_degrees.to_radians().cos().abs().max(0.25);
    GRID_OFFSETS
        .iter()
        .flat_map(|latitude_offset| {
            GRID_OFFSETS.iter().map(move |longitude_offset| {
                (
                    region.latitude_degrees + latitude_offset,
                    region.longitude_degrees + longitude_offset / longitude_scale,
                )
            })
        })
        .collect()
}

fn join_coordinates(values: impl Iterator<Item = f64>) -> String {
    values
        .map(|value| format!("{value:.4}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn age(now: DateTime<Utc>, accepted_at: DateTime<Utc>) -> Duration {
    (now - accepted_at).to_std().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn levels_and_regions_are_allowlisted() {
        assert_eq!(WindLevel::parse("500"), Some(WindLevel::Hpa500));
        assert_eq!(WindLevel::parse("450"), None);
        assert!(find_public_live_region_definition("ord").is_some());
        assert!(find_public_live_region_definition("world").is_none());
    }

    #[test]
    fn regional_grid_is_bounded_and_stable() {
        let region = find_public_live_region_definition("sfo").unwrap().region;
        let first = grid_points(region);
        assert_eq!(first, grid_points(region));
        assert_eq!(first.len(), 16);
        assert!(first.iter().all(|(lat, lon)| {
            (region.latitude_degrees - lat).abs() <= 1.21
                && (region.longitude_degrees - lon).abs() <= 1.7
        }));
    }

    #[test]
    fn provider_payload_is_sanitized_and_requires_one_consistent_grid() {
        let now = DateTime::parse_from_rfc3339("2026-07-22T00:50:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let records = (0..16)
            .map(|index| {
                serde_json::json!({
                    "latitude": 36.5 + index as f64 / 10.0,
                    "longitude": -123.5 + index as f64 / 10.0,
                    "current": {
                        "time": "2026-07-22T00:45",
                        "wind_speed_500hPa": 42.5,
                        "wind_direction_500hPa": 275.0,
                        "temperature_500hPa": -17.0
                    }
                })
            })
            .collect();
        let snapshot = parse_snapshot(
            Value::Array(records),
            "sfo",
            "San Francisco",
            WindLevel::Hpa500,
            "wind_speed_500hPa",
            "wind_direction_500hPa",
            now,
        )
        .unwrap();
        assert_eq!(snapshot.samples.len(), 16);
        assert_eq!(snapshot.level.pressure_hpa, Some(500));
        assert_eq!(
            snapshot.forecast_time.to_rfc3339(),
            "2026-07-22T00:45:00+00:00"
        );
        let serialized = serde_json::to_value(snapshot).unwrap();
        assert!(serialized.to_string().contains("speed_knots"));
        assert!(!serialized.to_string().contains("temperature"));
    }

    #[test]
    fn provider_payload_rejects_unbounded_values() {
        let records = (0..16)
            .map(|_| {
                serde_json::json!({
                    "latitude": 37.0,
                    "longitude": -122.0,
                    "current": {
                        "time": "2026-07-22T00:45",
                        "wind_speed_10m": 999.0,
                        "wind_direction_10m": 180.0
                    }
                })
            })
            .collect();
        assert!(matches!(
            parse_snapshot(
                Value::Array(records),
                "sfo",
                "San Francisco",
                WindLevel::Surface,
                "wind_speed_10m",
                "wind_direction_10m",
                Utc::now()
            ),
            Err(AtmosphereError::InvalidPayload("invalid wind speed"))
        ));
    }

    #[tokio::test]
    async fn cache_coalesces_refresh_work_and_retains_last_field_on_failure() {
        let (service, calls) =
            stub_service(1, Duration::from_millis(1), Duration::from_secs(60)).await;
        let first = service.snapshot("sfo", WindLevel::Surface).await.unwrap();
        tokio::time::sleep(Duration::from_millis(3)).await;
        let retained = service.snapshot("sfo", WindLevel::Surface).await.unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(first.state, "current");
        assert_eq!(retained.state, "degraded");
        assert!(retained.retained);
        assert_eq!(retained.samples.len(), 16);
        assert_eq!(retained.last_error_code, Some("provider_unavailable"));
    }

    #[tokio::test]
    async fn fresh_cache_prevents_duplicate_provider_requests() {
        let (service, calls) = stub_service(
            usize::MAX,
            Duration::from_secs(60),
            Duration::from_secs(120),
        )
        .await;
        service.snapshot("lax", WindLevel::Surface).await.unwrap();
        service.snapshot("lax", WindLevel::Surface).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[derive(Clone)]
    struct StubProvider {
        calls: Arc<AtomicUsize>,
        fail_after: usize,
    }

    async fn stub_service(
        fail_after: usize,
        refresh_after: Duration,
        retain_for: Duration,
    ) -> (AtmosphereService, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let state = StubProvider {
            calls: calls.clone(),
            fail_after,
        };
        tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route("/", get(stub_wind)).with_state(state),
            )
            .await
            .unwrap();
        });
        let service = AtmosphereService::new(
            Url::parse(&format!("http://{address}/")).unwrap(),
            refresh_after,
            retain_for,
        )
        .unwrap();
        (service, calls)
    }

    async fn stub_wind(State(state): State<StubProvider>) -> Response {
        let call = state.calls.fetch_add(1, Ordering::SeqCst);
        if call >= state.fail_after {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        Json(Value::Array(
            (0..16)
                .map(|index| {
                    serde_json::json!({
                        "latitude": 36.5 + index as f64 / 10.0,
                        "longitude": -123.5 + index as f64 / 10.0,
                        "current": {
                            "time": "2026-07-22T00:45",
                            "wind_speed_10m": 21.0,
                            "wind_direction_10m": 270.0
                        }
                    })
                })
                .collect(),
        ))
        .into_response()
    }
}
