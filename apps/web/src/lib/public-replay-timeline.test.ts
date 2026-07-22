import { describe, expect, it } from "vitest";
import {
  parsePublicReplayTimeline,
  replayPictureAt,
  replayTrailAt,
  type PublicReplayTimeline,
} from "./public-replay-timeline";

describe("public replay timeline", () => {
  it("parses a bounded timeline and interpolates one synchronized picture", () => {
    const timeline = parsePublicReplayTimeline(timelinePayload());
    const picture = replayPictureAt(timeline, 30_500);
    const aircraft = picture.aircraft[0];

    expect(aircraft.callsign).toBe("FT303");
    expect(aircraft.longitude_degrees).toBeCloseTo(-121.51, 2);
    expect(aircraft.altitude?.value).toBeCloseTo(27_500, 0);
    expect(aircraft.ground_speed?.value).toBeCloseTo(441.5, 1);
    expect(aircraft.quality).toBe("estimated");
    expect(aircraft.observed_at).toBe("2026-07-20T16:00:30.500Z");
    expect(replayTrailAt(timeline, "FT303", 30_500, aircraft)).toHaveLength(2);
  });

  it("resets deterministically and never reveals a flight before its first observation", () => {
    const timeline = parsePublicReplayTimeline(timelinePayload());
    expect(replayPictureAt(timeline, 0).aircraft).toEqual([]);
    expect(replayPictureAt(timeline, 1_000)).toEqual(replayPictureAt(timeline, 1_000));
    expect(replayPictureAt(timeline, 999_999).aircraft[0].altitude?.value).toBe(27_000);
  });

  it("rejects unbounded, out-of-order, and malformed timelines", () => {
    expect(() => parsePublicReplayTimeline({ ...timelinePayload(), duration_ms: 901_000 })).toThrow();
    expect(() => parsePublicReplayTimeline({
      ...timelinePayload(),
      observations: [...timelinePayload().observations].reverse(),
    })).toThrow(/out of order/i);
    expect(() => parsePublicReplayTimeline({
      ...timelinePayload(),
      observations: [{ ...timelinePayload().observations[0], longitude_degrees: 999 }],
    })).toThrow(/invalid observation/i);
  });
});

export function timelinePayload(): PublicReplayTimeline {
  return {
    schema_version: 1,
    scenario_id: "m1-operations-v1",
    start_time: "2026-07-20T16:00:00Z",
    end_time: "2026-07-20T16:01:00Z",
    duration_ms: 60_000,
    playback_speeds: [0.5, 1, 2],
    source: "portfolio deterministic replay",
    observations: [
      observation(1_000, -121.4, 37, 28_000, 315, 445),
      observation(60_000, -121.62, 37.18, 27_000, 315, 438),
    ],
  };
}

function observation(
  offsetMs: number,
  longitude: number,
  latitude: number,
  altitude: number,
  heading: number,
  speed: number,
) {
  return {
    callsign: "FT303",
    aircraft_registration: "N303FT",
    offset_ms: offsetMs,
    observed_at: new Date(Date.parse("2026-07-20T16:00:00Z") + offsetMs).toISOString(),
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    altitude: { value: altitude, unit: "feet" as const, reference: "mean_sea_level" },
    heading_true_degrees: heading,
    ground_speed: { value: speed, unit: "knots" as const },
    quality: "observed" as const,
  };
}
