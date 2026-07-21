import { describe, expect, it } from "vitest";
import type { PublicAircraft } from "@/lib/public-live-positions";
import {
  estimateTrajectory,
  MAX_TRAIL_POINTS,
  TRAIL_RETENTION_MS,
  updateTrajectoryHistory,
} from "./flight-trajectories";

const NOW = Date.parse("2026-07-21T22:30:00Z");

describe("flight trajectory policy", () => {
  it("accumulates ordered observations without duplicates or rewinds", () => {
    const first = aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6);
    const second = aircraftAt("2026-07-21T22:29:30Z", -122.3, 37.7);
    let history = updateTrajectoryHistory(new Map(), [first], NOW);
    history = updateTrajectoryHistory(history, [first], NOW);
    history = updateTrajectoryHistory(history, [second], NOW);
    history = updateTrajectoryHistory(history, [first], NOW);

    expect(history.get(first.id)).toEqual([
      pointAt(first.observed_at, -122.4, 37.6),
      pointAt(second.observed_at, -122.3, 37.7),
    ]);
  });

  it("bounds histories by age and point count while pruning missing aircraft", () => {
    let history = new Map<string, readonly ReturnType<typeof pointAt>[]>([
      ["missing", [pointAt(new Date(NOW - TRAIL_RETENTION_MS - 1).toISOString(), -120, 35)]],
    ]);
    for (let index = 0; index < MAX_TRAIL_POINTS + 5; index += 1) {
      history = updateTrajectoryHistory(history, [aircraftAt(
        new Date(NOW - 9 * 60_000 + index * 10_000).toISOString(),
        -122.5 + index / 100,
        37.5,
      )], NOW);
    }

    expect(history.has("missing")).toBe(false);
    expect(history.get("aircraft-1")).toHaveLength(MAX_TRAIL_POINTS);
    expect(history.get("aircraft-1")?.at(0)?.observed_at).toBe("2026-07-21T22:21:50.000Z");
  });

  it("projects supplied heading and speed along a geodesic five minutes ahead", () => {
    const northbound = aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6, 0, 60);
    const eastbound = aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6, 90, 120);

    const north = estimateTrajectory(northbound);
    const east = estimateTrajectory(eastbound);

    expect(north?.distance_nautical_miles).toBeCloseTo(5, 6);
    expect(north?.end.latitude_degrees).toBeGreaterThan(37.68);
    expect(north?.end.longitude_degrees).toBeCloseTo(-122.4, 4);
    expect(east?.distance_nautical_miles).toBeCloseTo(10, 6);
    expect(east?.end.longitude_degrees).toBeGreaterThan(-122.2);
    expect(east?.end.observed_at).toBe("2026-07-21T22:34:00.000Z");
  });

  it("does not estimate a trajectory without usable heading and speed", () => {
    expect(estimateTrajectory({
      ...aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6),
      heading_true_degrees: null,
    })).toBeNull();
    expect(estimateTrajectory({
      ...aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6),
      ground_speed: null,
    })).toBeNull();
    expect(estimateTrajectory({
      ...aircraftAt("2026-07-21T22:29:00Z", -122.4, 37.6),
      ground_speed: { value: Number.NaN, unit: "knots" },
    })).toBeNull();
    expect(estimateTrajectory({
      ...aircraftAt("not-a-time", -122.4, 37.6),
    })).toBeNull();
  });
});

function aircraftAt(
  observedAt: string,
  longitude: number,
  latitude: number,
  heading = 270,
  speed = 300,
): PublicAircraft {
  return {
    id: "aircraft-1",
    callsign: "UAL123",
    aircraft_registration: null,
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    altitude: null,
    heading_true_degrees: heading,
    ground_speed: { value: speed, unit: "knots" },
    quality: "observed",
    observed_at: observedAt,
    received_at: observedAt,
    provider: "adsb.lol",
  };
}

function pointAt(observedAt: string, longitude: number, latitude: number) {
  return {
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    observed_at: observedAt,
  };
}
