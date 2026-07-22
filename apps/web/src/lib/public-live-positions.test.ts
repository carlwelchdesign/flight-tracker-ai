import { describe, expect, it } from "vitest";
import { parsePublicLiveSnapshot } from "./public-live-positions";

const status = {
  enabled: true,
  provider: "adsb.lol",
  state: "current",
  best_effort: true,
  observed_at: "2026-07-21T20:00:00Z",
  last_success_at: "2026-07-21T20:00:00Z",
  newest_position_at: "2026-07-21T19:59:58Z",
  aircraft_count: 1,
  fresh_position_count: 1,
  stale_position_count: 0,
  stale_after_seconds: 30,
  region: { latitude_degrees: 37.62, longitude_degrees: -122.38, radius_nautical_miles: 25 },
  attribution: {
    text: "ADSB.lol ODbL",
    source_name: "ADSB.lol",
    source_url: "https://adsb.lol/",
    terms_label: "Open Database License (ODbL)",
    terms_url: "https://example.test/license",
  },
};

describe("parsePublicLiveSnapshot", () => {
  it("accepts the sanitized live aircraft contract", () => {
    const result = parsePublicLiveSnapshot({
      region_code: "sfo",
      region_name: "San Francisco",
      status,
      data: [{
        id: "flight-1",
        callsign: "UAL42",
        aircraft_registration: null,
        icao_hex: "a1b2c3",
        longitude_degrees: -122.3,
        latitude_degrees: 37.7,
        altitude: { value: 12000, unit: "feet", reference: "mean_sea_level" },
        heading_true_degrees: 90,
        ground_speed: { value: 280, unit: "knots" },
        quality: "observed",
        observed_at: "2026-07-21T19:59:58Z",
        received_at: "2026-07-21T20:00:00Z",
        provider: "adsb.lol",
      }],
    });

    expect(result.data[0].callsign).toBe("UAL42");
    expect(result.data[0].icao_hex).toBe("A1B2C3");
    expect(result.region_code).toBe("sfo");
    expect(result.status.state).toBe("current");
  });

  it("rejects invalid coordinates", () => {
    expect(() => parsePublicLiveSnapshot({
      region_code: "sfo",
      region_name: "San Francisco",
      status,
      data: [{
        id: "flight-1", callsign: null, aircraft_registration: null,
        longitude_degrees: 181, latitude_degrees: 37.7,
        observed_at: "2026-07-21T19:59:58Z", received_at: "2026-07-21T20:00:00Z",
        provider: "adsb.lol",
      }],
    })).toThrow("invalid aircraft");
  });

  it("rejects a non-ICAO aircraft identifier", () => {
    expect(() => parsePublicLiveSnapshot({
      region_code: "sfo", region_name: "San Francisco", status,
      data: [{
        id: "flight-1", callsign: "UAL42", aircraft_registration: null, icao_hex: "internal-record",
        longitude_degrees: -122.3, latitude_degrees: 37.7,
        observed_at: "2026-07-21T19:59:58Z", received_at: "2026-07-21T20:00:00Z", provider: "adsb.lol",
      }],
    })).toThrow("invalid aircraft");
  });
});
