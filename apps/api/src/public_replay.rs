use std::sync::LazyLock;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::{
    domain::{Altitude, SourceQuality, Speed},
    public_attention::PORTFOLIO_SCENARIO,
    replay::{ReplayScenario, ScenarioPayload},
};

const MAX_PUBLIC_REPLAY_DURATION_MS: u64 = 15 * 60 * 1_000;
const MAX_PUBLIC_REPLAY_OBSERVATIONS: usize = 100;

static PUBLIC_TIMELINE: LazyLock<Result<PublicReplayTimeline, String>> =
    LazyLock::new(build_public_replay_timeline);

#[derive(Debug, Clone, Serialize)]
struct PublicReplayTimeline {
    schema_version: u16,
    scenario_id: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    duration_ms: u64,
    playback_speeds: [f64; 3],
    source: &'static str,
    observations: Vec<PublicReplayObservation>,
}

#[derive(Debug, Clone, Serialize)]
struct PublicReplayObservation {
    callsign: String,
    aircraft_registration: String,
    offset_ms: u64,
    observed_at: DateTime<Utc>,
    longitude_degrees: f64,
    latitude_degrees: f64,
    altitude: Option<Altitude>,
    heading_true_degrees: Option<f64>,
    ground_speed: Option<Speed>,
    quality: SourceQuality,
}

pub fn public_replay_router() -> Router {
    Router::new().route("/api/public/replay/timeline", get(public_replay_timeline))
}

async fn public_replay_timeline() -> impl IntoResponse {
    match &*PUBLIC_TIMELINE {
        Ok(timeline) => (StatusCode::OK, Json(timeline.clone())).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "code": "public_replay_timeline_unavailable",
                    "message": "The deterministic replay timeline is unavailable"
                }
            })),
        )
            .into_response(),
    }
}

fn build_public_replay_timeline() -> Result<PublicReplayTimeline, String> {
    let scenario =
        ReplayScenario::from_json(PORTFOLIO_SCENARIO).map_err(|error| error.to_string())?;
    let duration_ms = scenario
        .events
        .iter()
        .map(|event| event.offset_ms)
        .max()
        .unwrap_or_default();
    if duration_ms == 0 || duration_ms > MAX_PUBLIC_REPLAY_DURATION_MS {
        return Err("portfolio replay duration is outside the public bound".into());
    }

    let mut observations = Vec::new();
    for event in &scenario.events {
        let ScenarioPayload::Position {
            flight_id,
            point,
            altitude,
            heading_true_degrees,
            ground_speed,
            quality,
        } = &event.payload
        else {
            continue;
        };
        let flight = scenario
            .flights
            .iter()
            .find(|flight| flight.id == *flight_id)
            .ok_or_else(|| "portfolio replay position references an unknown flight".to_string())?;
        observations.push(PublicReplayObservation {
            callsign: flight.callsign.clone(),
            aircraft_registration: flight.aircraft_registration.clone(),
            offset_ms: event.offset_ms,
            observed_at: scenario.start_time + Duration::milliseconds(event.offset_ms as i64),
            longitude_degrees: point.longitude_degrees.into(),
            latitude_degrees: point.latitude_degrees.into(),
            altitude: *altitude,
            heading_true_degrees: heading_true_degrees.map(Into::into),
            ground_speed: *ground_speed,
            quality: *quality,
        });
    }
    if observations.is_empty() || observations.len() > MAX_PUBLIC_REPLAY_OBSERVATIONS {
        return Err("portfolio replay observation count is outside the public bound".into());
    }

    Ok(PublicReplayTimeline {
        schema_version: 1,
        scenario_id: scenario.id,
        start_time: scenario.start_time,
        end_time: scenario.start_time + Duration::milliseconds(duration_ms as i64),
        duration_ms,
        playback_speeds: [0.5, 1.0, 2.0],
        source: "portfolio deterministic replay",
        observations,
    })
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn timeline_is_bounded_ordered_and_uses_scenario_observations() {
        let timeline = build_public_replay_timeline().unwrap();
        assert_eq!(timeline.duration_ms, 180_000);
        assert_eq!(timeline.observations.len(), 7);
        assert!(
            timeline
                .observations
                .windows(2)
                .all(|pair| pair[0].offset_ms <= pair[1].offset_ms)
        );
        let attention_frame = timeline
            .observations
            .iter()
            .find(|observation| observation.callsign == "FT303" && observation.offset_ms == 60_000)
            .unwrap();
        assert_eq!(attention_frame.altitude.unwrap().value, 27_000);
        assert_eq!(attention_frame.ground_speed.unwrap().value, 438.0);
        assert_eq!(attention_frame.heading_true_degrees, Some(315.0));
    }

    #[test]
    fn attention_flight_positions_follow_the_recorded_heading() {
        let timeline = build_public_replay_timeline().unwrap();
        let observations: Vec<_> = timeline
            .observations
            .iter()
            .filter(|observation| observation.callsign == "FT303")
            .collect();

        for pair in observations.windows(2) {
            let supplied_heading = pair[0].heading_true_degrees.unwrap();
            let segment_bearing = initial_bearing_degrees(pair[0], pair[1]);
            let difference = angular_difference_degrees(supplied_heading, segment_bearing);
            assert!(
                difference <= 5.0,
                "FT303 segment bearing {segment_bearing:.2} differs from supplied heading {supplied_heading:.2}",
            );
        }
    }

    #[test]
    fn serialized_timeline_excludes_internal_and_protected_fields() {
        let serialized = serde_json::to_string(&build_public_replay_timeline().unwrap()).unwrap();
        for forbidden in [
            "operator_id",
            "flight_id",
            "alert_id",
            "hazard_id",
            "route_id",
            "raw_payload",
            "provider_record_id",
            "dispatcher",
            "audit",
        ] {
            assert!(!serialized.contains(forbidden), "leaked {forbidden}");
        }
    }

    #[tokio::test]
    async fn public_timeline_route_requires_no_authentication() {
        let response = public_replay_router()
            .oneshot(
                Request::get("/api/public/replay/timeline")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["duration_ms"], 180_000);
    }

    fn initial_bearing_degrees(
        start: &PublicReplayObservation,
        end: &PublicReplayObservation,
    ) -> f64 {
        let start_latitude = start.latitude_degrees.to_radians();
        let end_latitude = end.latitude_degrees.to_radians();
        let longitude_delta = (end.longitude_degrees - start.longitude_degrees).to_radians();
        let y = longitude_delta.sin() * end_latitude.cos();
        let x = start_latitude.cos() * end_latitude.sin()
            - start_latitude.sin() * end_latitude.cos() * longitude_delta.cos();
        (y.atan2(x).to_degrees() + 360.0) % 360.0
    }

    fn angular_difference_degrees(first: f64, second: f64) -> f64 {
        ((first - second + 540.0) % 360.0 - 180.0).abs()
    }
}
