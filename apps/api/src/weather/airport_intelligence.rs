use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use super::noaa::{NoaaClient, NoaaClientConfig, RetryPolicy};

const TAF_TTL: Duration = Duration::from_secs(600);
const PIREP_TTL: Duration = Duration::from_secs(60);
const MAX_PIREPS: usize = 20;
const NEARBY_NAUTICAL_MILES: f64 = 100.0;
const TAF_STATIONS: [&str; 7] = ["KSFO", "KLAX", "KSEA", "KDEN", "KORD", "KATL", "KJFK"];

#[derive(Clone, Copy)]
struct Airport {
    code: &'static str,
    name: &'static str,
    latitude: f64,
    longitude: f64,
}

const AIRPORTS: [Airport; 7] = [
    Airport {
        code: "KSFO",
        name: "San Francisco International",
        latitude: 37.6196,
        longitude: -122.3656,
    },
    Airport {
        code: "KLAX",
        name: "Los Angeles International",
        latitude: 33.9425,
        longitude: -118.4081,
    },
    Airport {
        code: "KSEA",
        name: "Seattle-Tacoma International",
        latitude: 47.4490,
        longitude: -122.3093,
    },
    Airport {
        code: "KDEN",
        name: "Denver International",
        latitude: 39.8561,
        longitude: -104.6737,
    },
    Airport {
        code: "KORD",
        name: "Chicago O'Hare International",
        latitude: 41.9742,
        longitude: -87.9073,
    },
    Airport {
        code: "KATL",
        name: "Hartsfield-Jackson Atlanta International",
        latitude: 33.6407,
        longitude: -84.4277,
    },
    Airport {
        code: "KJFK",
        name: "John F. Kennedy International",
        latitude: 40.6413,
        longitude: -73.7781,
    },
];

#[derive(Clone)]
struct CacheEntry {
    accepted_at: DateTime<Utc>,
    value: Value,
}

#[derive(Default)]
struct Cache {
    tafs: Option<CacheEntry>,
    pireps: Option<CacheEntry>,
}

#[derive(Clone)]
struct AirportIntelligenceState {
    client: NoaaClient,
    cache: Arc<Mutex<Cache>>,
}

#[derive(Deserialize)]
struct AirportQuery {
    airport: Option<String>,
}

#[derive(Serialize)]
struct Snapshot {
    state: &'static str,
    generated_at: DateTime<Utc>,
    airport: AirportView,
    attribution: Attribution,
    taf: FeedView<TafView>,
    pireps: FeedView<Vec<PirepView>>,
    coverage_note: &'static str,
}

#[derive(Serialize)]
struct AirportView {
    code: &'static str,
    name: &'static str,
    latitude_degrees: f64,
    longitude_degrees: f64,
}
#[derive(Serialize)]
struct Attribution {
    text: &'static str,
    source_url: &'static str,
}
#[derive(Serialize)]
struct FeedView<T> {
    state: &'static str,
    accepted_at: Option<DateTime<Utc>>,
    data: Option<T>,
}
#[derive(Serialize)]
struct TafView {
    issue_time: DateTime<Utc>,
    valid_from: DateTime<Utc>,
    valid_to: DateTime<Utc>,
    periods: Vec<TafPeriod>,
}
#[derive(Serialize)]
struct TafPeriod {
    valid_from: DateTime<Utc>,
    valid_to: DateTime<Utc>,
    change: String,
    probability_percent: Option<i64>,
    wind_direction_degrees: Option<i64>,
    wind_speed_knots: Option<i64>,
    wind_gust_knots: Option<i64>,
    visibility: Option<String>,
    weather: Option<String>,
    clouds: Vec<CloudLayer>,
}
#[derive(Serialize)]
struct CloudLayer {
    coverage: String,
    base_feet_agl: Option<i64>,
}
#[derive(Serialize)]
struct PirepView {
    report_time: DateTime<Utc>,
    received_at: DateTime<Utc>,
    distance_nautical_miles: f64,
    altitude_feet: Option<i64>,
    altitude_context: Option<String>,
    report_type: String,
    aircraft_type: Option<String>,
    turbulence: Option<String>,
    icing: Option<String>,
    clouds: Option<String>,
    wind: Option<WindView>,
    temperature_celsius: Option<f64>,
    weather: Option<String>,
    location_available: bool,
}
#[derive(Serialize)]
struct WindView {
    direction_degrees: i64,
    speed_knots: i64,
}

pub fn airport_intelligence_router() -> Router {
    let client = NoaaClient::new(NoaaClientConfig {
        base_url: Url::parse("https://aviationweather.gov/").expect("fixed NOAA URL is valid"),
        user_agent:
            "FlightTrackerAI-Portfolio/1.0 (+https://github.com/carlwelchdesign/flight-tracker-ai)"
                .into(),
        connect_timeout: Duration::from_secs(3),
        request_timeout: Duration::from_secs(8),
        retry: RetryPolicy::default(),
    })
    .expect("fixed NOAA client configuration is valid");
    Router::new()
        .route(
            "/api/public/airport-intelligence",
            get(get_airport_intelligence),
        )
        .with_state(AirportIntelligenceState {
            client,
            cache: Arc::new(Mutex::new(Cache::default())),
        })
}

async fn get_airport_intelligence(
    State(state): State<AirportIntelligenceState>,
    Query(query): Query<AirportQuery>,
) -> Response {
    let code = query
        .airport
        .unwrap_or_else(|| "KSFO".into())
        .to_ascii_uppercase();
    let Some(airport) = AIRPORTS
        .iter()
        .copied()
        .find(|airport| airport.code == code)
    else {
        return (StatusCode::BAD_REQUEST, [(header::CACHE_CONTROL, "no-store")], Json(serde_json::json!({"error":{"code":"invalid_airport","message":"Choose an allowlisted airport"}}))).into_response();
    };
    let now = Utc::now();
    let mut cache = state.cache.lock().await;
    let taf_current = cache
        .tafs
        .as_ref()
        .is_some_and(|entry| age(now, entry.accepted_at) <= TAF_TTL);
    let pirep_current = cache
        .pireps
        .as_ref()
        .is_some_and(|entry| age(now, entry.accepted_at) <= PIREP_TTL);
    let mut taf_failed = false;
    let mut pirep_failed = false;
    if !taf_current {
        match state.client.fetch_tafs(&TAF_STATIONS).await {
            Ok(Some(value)) if value.is_array() => {
                cache.tafs = Some(CacheEntry {
                    accepted_at: now,
                    value,
                })
            }
            Ok(None) => {
                cache.tafs = Some(CacheEntry {
                    accepted_at: now,
                    value: Value::Array(Vec::new()),
                })
            }
            _ => taf_failed = true,
        }
    }
    if !pirep_current {
        match state.client.fetch_pireps().await {
            Ok(Some(value)) if value.is_array() => {
                cache.pireps = Some(CacheEntry {
                    accepted_at: now,
                    value,
                })
            }
            Ok(None) => {
                cache.pireps = Some(CacheEntry {
                    accepted_at: now,
                    value: Value::Array(Vec::new()),
                })
            }
            _ => pirep_failed = true,
        }
    }
    let taf = feed_view(cache.tafs.as_ref(), taf_failed, |value| {
        parse_taf(value, airport.code)
    });
    let pireps = feed_view(cache.pireps.as_ref(), pirep_failed, |value| {
        Some(parse_pireps(value, airport))
    });
    let state_name = match (taf.state, pireps.state) {
        ("unavailable", "unavailable") => "unavailable",
        ("current", "current") => "current",
        ("retained", _) | (_, "retained") => "retained",
        _ => "partial",
    };
    (StatusCode::OK, [(header::CACHE_CONTROL, "public, max-age=30, stale-while-revalidate=60")], Json(Snapshot {
        state: state_name, generated_at: now,
        airport: AirportView { code: airport.code, name: airport.name, latitude_degrees: airport.latitude, longitude_degrees: airport.longitude },
        attribution: Attribution { text: "Forecasts and pilot reports from NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
        taf, pireps,
        coverage_note: "Nearby pilot reports are sparse, voluntary reports within 100 NM. Reports without a usable location are excluded; absence does not mean conditions are absent, and reports are not attributed to any selected flight.",
    })).into_response()
}

fn feed_view<T>(
    entry: Option<&CacheEntry>,
    failed: bool,
    parse: impl FnOnce(&Value) -> Option<T>,
) -> FeedView<T> {
    match entry.and_then(|entry| parse(&entry.value).map(|data| (entry.accepted_at, data))) {
        Some((accepted_at, data)) => FeedView {
            state: if failed { "retained" } else { "current" },
            accepted_at: Some(accepted_at),
            data: Some(data),
        },
        None => FeedView {
            state: "unavailable",
            accepted_at: entry.map(|entry| entry.accepted_at),
            data: None,
        },
    }
}

fn parse_taf(value: &Value, airport: &str) -> Option<TafView> {
    let taf = value
        .as_array()?
        .iter()
        .find(|item| item.get("icaoId").and_then(Value::as_str) == Some(airport))?;
    Some(TafView {
        issue_time: time_string(taf.get("issueTime")?)?,
        valid_from: epoch(taf.get("validTimeFrom")?)?,
        valid_to: epoch(taf.get("validTimeTo")?)?,
        periods: taf
            .get("fcsts")?
            .as_array()?
            .iter()
            .filter_map(|period| {
                Some(TafPeriod {
                    valid_from: epoch(period.get("timeFrom")?)?,
                    valid_to: epoch(period.get("timeTo")?)?,
                    change: period
                        .get("fcstChange")
                        .and_then(Value::as_str)
                        .unwrap_or("BASE")
                        .to_owned(),
                    probability_percent: period.get("probability").and_then(Value::as_i64),
                    wind_direction_degrees: period.get("wdir").and_then(Value::as_i64),
                    wind_speed_knots: period.get("wspd").and_then(Value::as_i64),
                    wind_gust_knots: period.get("wgst").and_then(Value::as_i64),
                    visibility: period
                        .get("visib")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    weather: period
                        .get("wxString")
                        .and_then(Value::as_str)
                        .filter(|v| !v.is_empty())
                        .map(str::to_owned),
                    clouds: period
                        .get("clouds")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|cloud| {
                            Some(CloudLayer {
                                coverage: cloud.get("cover")?.as_str()?.to_owned(),
                                base_feet_agl: cloud.get("base").and_then(Value::as_i64),
                            })
                        })
                        .collect(),
                })
            })
            .take(16)
            .collect(),
    })
}

fn parse_pireps(value: &Value, airport: Airport) -> Vec<PirepView> {
    let mut reports: Vec<_> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|report| {
            let lat = report.get("lat").and_then(Value::as_f64);
            let lon = report.get("lon").and_then(Value::as_f64);
            let distance = match (lat, lon) {
                (Some(lat), Some(lon)) => {
                    haversine_nm(airport.latitude, airport.longitude, lat, lon)
                }
                _ => return None,
            };
            if distance > NEARBY_NAUTICAL_MILES {
                return None;
            }
            Some(PirepView {
                report_time: epoch(report.get("obsTime")?)?,
                received_at: time_string(report.get("receiptTime")?)?,
                distance_nautical_miles: (distance * 10.0).round() / 10.0,
                altitude_feet: report
                    .get("fltLvl")
                    .and_then(Value::as_i64)
                    .map(|level| level * 100),
                altitude_context: text(report, "fltLvlType"),
                report_type: text(report, "pirepType").unwrap_or_else(|| "PIREP".into()),
                aircraft_type: text(report, "acType"),
                turbulence: joined(report, &["tbInt1", "tbType1", "tbFreq1"]),
                icing: joined(report, &["icgInt1", "icgType1"]),
                clouds: report
                    .get("clouds")
                    .filter(|value| !value.is_null())
                    .map(Value::to_string),
                wind: match (
                    report.get("wdir").and_then(Value::as_i64),
                    report.get("wspd").and_then(Value::as_i64),
                ) {
                    (Some(direction_degrees), Some(speed_knots)) => Some(WindView {
                        direction_degrees,
                        speed_knots,
                    }),
                    _ => None,
                },
                temperature_celsius: report.get("temp").and_then(Value::as_f64),
                weather: text(report, "wxString"),
                location_available: true,
            })
        })
        .collect();
    reports.sort_by(|a, b| {
        a.distance_nautical_miles
            .total_cmp(&b.distance_nautical_miles)
            .then_with(|| b.report_time.cmp(&a.report_time))
    });
    reports.truncate(MAX_PIREPS);
    reports
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
fn joined(value: &Value, keys: &[&str]) -> Option<String> {
    let parts: Vec<_> = keys.iter().filter_map(|key| text(value, key)).collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}
fn epoch(value: &Value) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(value.as_i64()?, 0).single()
}
fn time_string(value: &Value) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.as_str()?)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}
fn age(now: DateTime<Utc>, then: DateTime<Utc>) -> Duration {
    now.signed_duration_since(then).to_std().unwrap_or_default()
}
fn haversine_nm(a_lat: f64, a_lon: f64, b_lat: f64, b_lon: f64) -> f64 {
    let (a_lat, b_lat) = (a_lat.to_radians(), b_lat.to_radians());
    let d_lat = b_lat - a_lat;
    let d_lon = (b_lon - a_lon).to_radians();
    let h = (d_lat / 2.0).sin().powi(2) + a_lat.cos() * b_lat.cos() * (d_lon / 2.0).sin().powi(2);
    3440.065 * 2.0 * h.sqrt().asin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use serde_json::json;
    use tower::ServiceExt;

    #[test]
    fn taf_parser_keeps_period_times_units_and_changes() {
        let value = json!([{"icaoId":"KSFO","issueTime":"2026-07-22T08:53:00Z","validTimeFrom":1784710800,"validTimeTo":1784808000,"fcsts":[{"timeFrom":1784710800,"timeTo":1784721600,"fcstChange":"TEMPO","probability":30,"wdir":220,"wspd":15,"visib":"6+","clouds":[{"cover":"BKN","base":1200}]}]}]);
        let taf = parse_taf(&value, "KSFO").unwrap();
        assert_eq!(taf.periods.len(), 1);
        assert_eq!(taf.periods[0].change, "TEMPO");
        assert_eq!(taf.periods[0].probability_percent, Some(30));
        assert_eq!(taf.periods[0].clouds[0].base_feet_agl, Some(1200));
    }

    #[test]
    fn pireps_are_distance_filtered_sorted_and_bounded() {
        let reports = json!((0..25).map(|index| json!({"receiptTime":"2026-07-22T09:30:00Z","obsTime":1784711700,"lat":37.62 + index as f64 / 1000.0,"lon":-122.36,"fltLvl":70,"pirepType":"PIREP","tbInt1":"LGT"})).collect::<Vec<_>>());
        let parsed = parse_pireps(&reports, AIRPORTS[0]);
        assert_eq!(parsed.len(), MAX_PIREPS);
        assert!(
            parsed
                .windows(2)
                .all(|pair| pair[0].distance_nautical_miles <= pair[1].distance_nautical_miles)
        );
        assert_eq!(parsed[0].altitude_feet, Some(7000));
    }

    #[tokio::test]
    async fn public_route_rejects_arbitrary_airports_and_query_shapes_before_fetching() {
        let response = airport_intelligence_router()
            .oneshot(
                Request::get("/api/public/airport-intelligence?airport=EGLL&bbox=world&age=15")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }

    #[test]
    fn failed_refresh_labels_last_accepted_picture_as_retained() {
        let entry = CacheEntry {
            accepted_at: Utc::now(),
            value: json!([]),
        };
        let retained = feed_view(Some(&entry), true, |_| Some(Vec::<PirepView>::new()));
        assert_eq!(retained.state, "retained");
        assert!(retained.accepted_at.is_some());
        let unavailable = feed_view::<Vec<PirepView>>(None, true, |_| Some(Vec::new()));
        assert_eq!(unavailable.state, "unavailable");
    }
}
