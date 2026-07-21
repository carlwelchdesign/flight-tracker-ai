import { describe, expect, it } from "vitest";

import { parseLivePositionStatus } from "./live-positions-api";

const status = {
  enabled: true,
  provider: "adsb.lol",
  feed: "point",
  state: "current",
  best_effort: true,
  observed_at: "2026-07-21T17:16:02Z",
  last_attempt_at: "2026-07-21T17:16:02Z",
  last_success_at: "2026-07-21T17:16:02Z",
  newest_position_at: "2026-07-21T17:16:00Z",
  consecutive_failures: 0,
  aircraft_count: 12,
  fresh_position_count: 10,
  stale_position_count: 2,
  rejected_record_count: 1,
  missing_callsign_count: 3,
  stale_after_seconds: 30,
  last_error_code: null,
  region: {
    latitude_degrees: 37.62,
    longitude_degrees: -122.38,
    radius_nautical_miles: 25,
  },
  attribution: {
    text: "Contains information from ADSB.lol, available under the Open Database License (ODbL).",
    source_url: "https://adsb.lol/",
    license_url: "https://opendatacommons.org/licenses/odbl/1-0/",
  },
};

describe("live position transport", () => {
  it("accepts the bounded provider-neutral source status", () => {
    expect(parseLivePositionStatus(status)).toEqual(status);
  });

  it("rejects malformed counts and attribution before rendering", () => {
    expect(() => parseLivePositionStatus({ ...status, aircraft_count: -1 })).toThrow(
      /unexpected status payload/,
    );
    expect(() =>
      parseLivePositionStatus({ ...status, attribution: { text: "missing links" } }),
    ).toThrow(/unexpected status payload/);
  });
});
