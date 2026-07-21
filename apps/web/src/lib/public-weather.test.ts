import { describe, expect, it } from "vitest";
import { parsePublicWeatherSnapshot } from "./public-weather";

describe("public weather boundary", () => {
  it("accepts the bounded sanitized NOAA snapshot", () => {
    const parsed = parsePublicWeatherSnapshot(snapshot());

    expect(parsed.state).toBe("current");
    expect(parsed.hazards[0].hazard_type).toBe("convective");
    expect(parsed.observations[0].station_code).toBe("KSFO");
    expect(parsed.hazards[0]).not.toHaveProperty("operator_id");
    expect(parsed.hazards[0]).not.toHaveProperty("envelope_id");
  });

  it.each([
    { generated_at: "not-a-time" },
    { state: "healthy" },
    { hazards: [{ ...hazard(), severity: "catastrophic" }] },
    { hazards: [{ ...hazard(), valid_from: "not-a-time" }] },
    { hazards: [{ ...hazard(), footprint: { exterior: [{ longitude_degrees: 0, latitude_degrees: 0 }] } }] },
    { observations: [{ ...observation(), point: { longitude_degrees: 400, latitude_degrees: 0 } }] },
    { observations: [{ ...observation(), wind_speed: { value: -1, unit: "knots" } }] },
    { observations: [{ ...observation(), flight_category: "danger" }] },
    { sources: [{ ...source(), state: "fine" }] },
  ])("rejects malformed public weather evidence %#", (change) => {
    expect(() => parsePublicWeatherSnapshot({ ...snapshot(), ...change })).toThrow();
  });
});

function snapshot() {
  return {
    state: "current",
    generated_at: "2026-07-21T23:00:00Z",
    attribution: { text: "NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
    sources: [source()],
    hazards: [hazard()],
    observations: [observation()],
  };
}

function source() {
  return {
    provider: "noaa-awc", feed: "metar", state: "healthy",
    observed_at: "2026-07-21T23:00:00Z", last_success_at: "2026-07-21T23:00:00Z",
    newest_event_at: "2026-07-21T22:55:00Z", stale_after_seconds: 900, last_error_code: null,
  };
}

function hazard() {
  return {
    id: "hazard-1", source: { provider: "noaa-awc", feed: "airsigmet" }, status: "active",
    issued_at: "2026-07-21T22:30:00Z", hazard_type: "convective", severity: "significant",
    valid_from: "2026-07-21T22:30:00Z", valid_to: "2026-07-22T00:30:00Z", altitude_band: null,
    footprint: { exterior: [
      { longitude_degrees: -123, latitude_degrees: 37 },
      { longitude_degrees: -121, latitude_degrees: 37 },
      { longitude_degrees: -121, latitude_degrees: 39 },
      { longitude_degrees: -123, latitude_degrees: 37 },
    ] },
  };
}

function observation() {
  return {
    id: "observation-1", source: { provider: "noaa-awc", feed: "metar" },
    observed_at: "2026-07-21T22:55:00Z", received_at: "2026-07-21T22:56:00Z",
    station_code: "KSFO", report_type: "METAR",
    point: { longitude_degrees: -122.375, latitude_degrees: 37.619 },
    wind_direction_true_degrees: 280, wind_speed: { value: 15, unit: "knots" }, wind_gust: null,
    visibility_statute_miles: 10, visibility_greater_than: false,
    ceiling: { value: 1200, unit: "feet", reference: "above_ground_level" }, flight_category: "visual",
  };
}
