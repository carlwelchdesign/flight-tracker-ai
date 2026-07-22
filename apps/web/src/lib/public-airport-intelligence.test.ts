import { describe, expect, it } from "vitest";
import { parsePublicAirportIntelligence } from "./public-airport-intelligence";

const payload = {
  state: "current", generated_at: "2026-07-22T10:00:00Z",
  airport: { code: "KSFO", name: "San Francisco International", latitude_degrees: 37.62, longitude_degrees: -122.36 },
  attribution: { text: "NOAA", source_url: "https://aviationweather.gov/" },
  taf: { state: "current", accepted_at: "2026-07-22T10:00:00Z", data: { issue_time: "2026-07-22T09:00:00Z", valid_from: "2026-07-22T09:00:00Z", valid_to: "2026-07-23T12:00:00Z", periods: [{ valid_from: "2026-07-22T09:00:00Z", valid_to: "2026-07-22T12:00:00Z", change: "BASE", probability_percent: null, wind_direction_degrees: 220, wind_speed_knots: 15, wind_gust_knots: null, visibility: "6+", weather: null, clouds: [] }] } },
  pireps: { state: "current", accepted_at: "2026-07-22T10:00:00Z", data: [{ report_time: "2026-07-22T09:30:00Z", received_at: "2026-07-22T09:31:00Z", distance_nautical_miles: 12.4, altitude_feet: 7000, altitude_context: "OTHER", report_type: "PIREP", aircraft_type: "B737", turbulence: "LGT", icing: null, clouds: null, wind: null, temperature_celsius: null, weather: null, location_available: true }] },
  coverage_note: "Sparse voluntary reports",
};

describe("parsePublicAirportIntelligence", () => {
  it("accepts bounded TAF periods and nearby PIREPs", () => expect(parsePublicAirportIntelligence(payload).pireps.data).toHaveLength(1));
  it("rejects reports beyond the public nearby radius", () => expect(() => parsePublicAirportIntelligence({ ...payload, pireps: { ...payload.pireps, data: [{ ...payload.pireps.data[0], distance_nautical_miles: 101 }] } })).toThrow());
  it("rejects unbounded report arrays", () => expect(() => parsePublicAirportIntelligence({ ...payload, pireps: { ...payload.pireps, data: Array.from({ length: 21 }, () => payload.pireps.data[0]) } })).toThrow());
  it("rejects malformed forecast time evidence", () => expect(() => parsePublicAirportIntelligence({ ...payload, taf: { ...payload.taf, data: { ...payload.taf.data, issue_time: "not-a-time" } } })).toThrow());
});
