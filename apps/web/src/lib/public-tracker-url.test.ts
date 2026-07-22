import { describe, expect, it } from "vitest";
import {
  aircraftMatchesKey,
  aircraftUrlKey,
  defaultPublicTrackerUrlState,
  parsePublicTrackerUrl,
  serializePublicTrackerUrl,
} from "./public-tracker-url";

describe("public tracker URL state", () => {
  it("round trips bounded replay, map, aircraft, region, and weather state", () => {
    const state = {
      ...defaultPublicTrackerUrlState(),
      mode: "replay" as const,
      regionCode: "lax",
      aircraftKey: "FT303",
      replayElapsedMs: 121_000,
      mapView: { longitude: -121.62, latitude: 37.18, zoom: 8.25, bearing: -15, pitch: 30 },
      weather: {
        observations: true, hazards: false, radar: false, satellite: true,
        surfaceWind: true, modelWind: true, windLevel: "300" as const,
      },
    };
    expect(parsePublicTrackerUrl(serializePublicTrackerUrl(state))).toEqual(state);
  });

  it("normalizes unknown, oversized, and out-of-range state to useful defaults", () => {
    expect(parsePublicTrackerUrl(`?${"x".repeat(1_025)}`)).toEqual(defaultPublicTrackerUrlState());
    expect(parsePublicTrackerUrl("?mode=replay&scenario=unknown&region=world&aircraft=secret_value&t=-1&view=500,95,99,999,90&level=1000"))
      .toEqual(defaultPublicTrackerUrlState());
  });

  it("uses only displayed callsign or registration as a share key", () => {
    expect(aircraftUrlKey({ callsign: " ual123 ", aircraft_registration: "N123UA" })).toBe("UAL123");
    expect(aircraftUrlKey({ callsign: null, aircraft_registration: null, icao_hex: "A1B2C3" })).toBe("A1B2C3");
    expect(aircraftMatchesKey({ callsign: null, aircraft_registration: null, icao_hex: "A1B2C3" }, "A1B2C3")).toBe(true);
    expect(aircraftMatchesKey({ callsign: null, aircraft_registration: "N123UA" }, "N123UA")).toBe(true);
    expect(aircraftUrlKey({ callsign: null, aircraft_registration: null })).toBeNull();
  });
});
