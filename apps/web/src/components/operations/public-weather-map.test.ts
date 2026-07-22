import { describe, expect, it } from "vitest";
import type { PublicWeatherSnapshot } from "@/lib/public-weather";
import { hazardLifecycle, selectedWeather, weatherGeoJson } from "./public-weather-map";

describe("public weather map policy", () => {
  it("builds independently toggleable hazard and METAR features", () => {
    const snapshot = weatherSnapshot();

    expect(weatherGeoJson(snapshot, true, true, null).features).toHaveLength(2);
    expect(weatherGeoJson(snapshot, false, true, null).features.map((item) => item.properties.kind))
      .toEqual(["observation"]);
    expect(weatherGeoJson(snapshot, true, false, null).features.map((item) => item.properties.kind))
      .toEqual(["hazard"]);
  });

  it("marks only the selected evidence feature", () => {
    const snapshot = weatherSnapshot();
    const selection = { kind: "observation" as const, id: "observation-1" };
    const features = weatherGeoJson(snapshot, true, true, selection).features;

    expect(features.find((item) => item.properties.id === "observation-1")?.properties.selected).toBe(true);
    expect(selectedWeather(snapshot, selection)).toMatchObject({ station_code: "KSFO" });
  });

  it("distinguishes active, upcoming, expired, and cancelled hazards", () => {
    const now = Date.parse("2026-07-21T23:00:00Z");
    expect(hazardLifecycle({ status: "active", valid_from: "2026-07-21T22:00:00Z", valid_to: "2026-07-22T00:00:00Z" }, now)).toBe("active");
    expect(hazardLifecycle({ status: "active", valid_from: "2026-07-22T01:00:00Z", valid_to: "2026-07-22T02:00:00Z" }, now)).toBe("upcoming");
    expect(hazardLifecycle({ status: "active", valid_from: "2026-07-21T20:00:00Z", valid_to: "2026-07-21T21:00:00Z" }, now)).toBe("expired");
    expect(hazardLifecycle({ status: "cancelled", valid_from: "2026-07-21T22:00:00Z", valid_to: "2026-07-22T00:00:00Z" }, now)).toBe("cancelled");
  });
});

function weatherSnapshot(): PublicWeatherSnapshot {
  return {
    state: "current",
    generated_at: "2026-07-21T23:00:00Z",
    attribution: { text: "NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
    sources: [],
    hazards: [{
      id: "hazard-1", source: { provider: "noaa-awc", feed: "airsigmet" }, status: "active",
      issued_at: "2026-07-21T22:00:00Z", hazard_type: "convective", severity: "significant",
      valid_from: "2026-07-21T22:00:00Z", valid_to: "2026-07-22T00:00:00Z", altitude_band: null,
      footprint: { exterior: [
        { longitude_degrees: -123, latitude_degrees: 37 }, { longitude_degrees: -121, latitude_degrees: 37 },
        { longitude_degrees: -121, latitude_degrees: 39 }, { longitude_degrees: -123, latitude_degrees: 37 },
      ] },
    }],
    observations: [{
      id: "observation-1", source: { provider: "noaa-awc", feed: "metar" },
      observed_at: "2026-07-21T22:55:00Z", received_at: "2026-07-21T22:56:00Z",
      station_code: "KSFO", report_type: "METAR",
      point: { longitude_degrees: -122.375, latitude_degrees: 37.619 },
      wind_direction_true_degrees: 280, wind_speed: { value: 15, unit: "knots" }, wind_gust: null,
      visibility_statute_miles: 10, visibility_greater_than: false, ceiling: null, flight_category: "visual",
    }],
  };
}
