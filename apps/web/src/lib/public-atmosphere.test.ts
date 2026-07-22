import { describe, expect, it } from "vitest";
import { parsePublicWindField } from "./public-atmosphere";

describe("public atmospheric wind boundary", () => {
  it("accepts a bounded attributed grid", () => {
    const parsed = parsePublicWindField(snapshot());
    expect(parsed.level.code).toBe("500");
    expect(parsed.samples).toHaveLength(2);
  });

  it.each([
    { samples: [] },
    { samples: [{ latitude_degrees: 91, longitude_degrees: 0, speed_knots: 10, direction_from_degrees: 90 }] },
    { attribution: { ...snapshot().attribution, source_url: "http://example.test" } },
    { attribution: { ...snapshot().attribution, license_url: "http://example.test" } },
    { level: { ...snapshot().level, code: "450" } },
  ])("rejects malformed or unbounded wind evidence %#", (change) => {
    expect(() => parsePublicWindField({ ...snapshot(), ...change })).toThrow();
  });
});

function snapshot() {
  return {
    state: "current",
    retained: false,
    region_code: "sfo",
    region_name: "San Francisco",
    level: { code: "500", label: "500 hPa", pressure_hpa: 500, approximate_altitude_feet: 18_400 },
    generated_at: "2026-07-22T00:55:00Z",
    forecast_time: "2026-07-22T00:45:00Z",
    last_success_at: "2026-07-22T00:55:00Z",
    last_error_code: null,
    attribution: {
      provider: "Open-Meteo",
      model: "NOAA GFS / HRRR",
      source_url: "https://open-meteo.com/",
      license_url: "https://open-meteo.com/en/license",
      text: "NOAA GFS/HRRR model data delivered by Open-Meteo",
    },
    samples: [
      { latitude_degrees: 37, longitude_degrees: -123, speed_knots: 42, direction_from_degrees: 275 },
      { latitude_degrees: 38, longitude_degrees: -122, speed_knots: 39, direction_from_degrees: 281 },
    ],
  };
}
