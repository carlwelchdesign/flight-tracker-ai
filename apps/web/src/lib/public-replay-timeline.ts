import type { TrajectoryPoint } from "@/lib/flight-trajectories";
import type { PublicAircraft } from "@/lib/public-live-positions";

export type PublicReplayObservation = {
  callsign: string;
  aircraft_registration: string;
  offset_ms: number;
  observed_at: string;
  longitude_degrees: number;
  latitude_degrees: number;
  altitude: { value: number; unit: "feet" | "meters"; reference: string } | null;
  heading_true_degrees: number | null;
  ground_speed: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  quality: "observed" | "fused" | "estimated" | "unknown";
};

export type PublicReplayTimeline = {
  schema_version: 1;
  scenario_id: string;
  start_time: string;
  end_time: string;
  duration_ms: number;
  playback_speeds: number[];
  source: string;
  observations: PublicReplayObservation[];
};

export type ReplayPicture = {
  aircraft: PublicAircraft[];
  observationsByCallsign: ReadonlyMap<string, readonly PublicReplayObservation[]>;
};

const MAX_DURATION_MS = 15 * 60 * 1_000;
const MAX_OBSERVATIONS = 100;
const ALLOWED_SPEEDS = [0.5, 1, 2] as const;

export function parsePublicReplayTimeline(value: unknown): PublicReplayTimeline {
  if (
    !isRecord(value) ||
    value.schema_version !== 1 ||
    !isBoundedString(value.scenario_id, 64) ||
    !isTimestamp(value.start_time) ||
    !isTimestamp(value.end_time) ||
    !isIntegerInRange(value.duration_ms, 1, MAX_DURATION_MS) ||
    !Array.isArray(value.playback_speeds) ||
    value.playback_speeds.length === 0 ||
    value.playback_speeds.some((speed) => !ALLOWED_SPEEDS.includes(speed as 0.5 | 1 | 2)) ||
    !isBoundedString(value.source, 80) ||
    !Array.isArray(value.observations) ||
    value.observations.length === 0 ||
    value.observations.length > MAX_OBSERVATIONS
  ) {
    throw new Error("Public replay returned an unexpected timeline");
  }
  const durationMs = value.duration_ms;
  const observations = value.observations.map((observation) => parseObservation(observation, durationMs));
  if (observations.some((observation, index) => index > 0 && observations[index - 1].offset_ms > observation.offset_ms)) {
    throw new Error("Public replay observations are out of order");
  }
  return {
    schema_version: 1,
    scenario_id: value.scenario_id,
    start_time: value.start_time,
    end_time: value.end_time,
    duration_ms: durationMs,
    playback_speeds: [...new Set(value.playback_speeds as number[])],
    source: value.source,
    observations,
  };
}

export function replayPictureAt(timeline: PublicReplayTimeline, elapsedMs: number): ReplayPicture {
  const boundedElapsed = Math.min(timeline.duration_ms, Math.max(0, elapsedMs));
  const grouped = groupObservations(timeline.observations);
  const aircraft = [...grouped.entries()].flatMap(([callsign, observations]) => {
    const previous = observations.findLast((observation) => observation.offset_ms <= boundedElapsed);
    if (!previous) return [];
    const next = observations.find((observation) => observation.offset_ms > boundedElapsed);
    const fraction = next
      ? (boundedElapsed - previous.offset_ms) / (next.offset_ms - previous.offset_ms)
      : 0;
    const interpolated = Boolean(next && fraction > 0);
    const scenarioTime = new Date(Date.parse(timeline.start_time) + boundedElapsed).toISOString();
    return [{
      id: `replay:${callsign}`,
      callsign,
      aircraft_registration: previous.aircraft_registration,
      icao_hex: null,
      longitude_degrees: next ? interpolate(previous.longitude_degrees, next.longitude_degrees, fraction) : previous.longitude_degrees,
      latitude_degrees: next ? interpolate(previous.latitude_degrees, next.latitude_degrees, fraction) : previous.latitude_degrees,
      altitude: interpolateMeasurement(previous.altitude, next?.altitude ?? null, fraction),
      heading_true_degrees: interpolateHeading(previous.heading_true_degrees, next?.heading_true_degrees ?? null, fraction),
      ground_speed: interpolateMeasurement(previous.ground_speed, next?.ground_speed ?? null, fraction),
      quality: interpolated ? "estimated" : previous.quality,
      observed_at: scenarioTime,
      received_at: previous.observed_at,
      provider: interpolated ? "portfolio.replay.interpolated" : "portfolio.replay.observed",
    } satisfies PublicAircraft];
  });
  return { aircraft, observationsByCallsign: grouped };
}

export function replayTrailAt(
  timeline: PublicReplayTimeline,
  callsign: string,
  elapsedMs: number,
  current: PublicAircraft | null,
): TrajectoryPoint[] {
  const observed = timeline.observations
    .filter((observation) => observation.callsign === callsign && observation.offset_ms <= elapsedMs)
    .map((observation) => ({
      longitude_degrees: observation.longitude_degrees,
      latitude_degrees: observation.latitude_degrees,
      observed_at: observation.observed_at,
    }));
  if (current && current.quality === "estimated") {
    observed.push({
      longitude_degrees: current.longitude_degrees,
      latitude_degrees: current.latitude_degrees,
      observed_at: current.observed_at,
    });
  }
  return observed;
}

function groupObservations(observations: readonly PublicReplayObservation[]) {
  const grouped = new Map<string, PublicReplayObservation[]>();
  for (const observation of observations) {
    const values = grouped.get(observation.callsign) ?? [];
    values.push(observation);
    grouped.set(observation.callsign, values);
  }
  return grouped;
}

function parseObservation(value: unknown, durationMs: number): PublicReplayObservation {
  if (
    !isRecord(value) ||
    !isBoundedString(value.callsign, 16) ||
    !isBoundedString(value.aircraft_registration, 16) ||
    !isIntegerInRange(value.offset_ms, 0, durationMs) ||
    !isTimestamp(value.observed_at) ||
    !isFiniteInRange(value.longitude_degrees, -180, 180) ||
    !isFiniteInRange(value.latitude_degrees, -90, 90) ||
    !isAltitude(value.altitude) ||
    !(value.heading_true_degrees === null || isFiniteInRange(value.heading_true_degrees, 0, 359.9999)) ||
    !isSpeed(value.ground_speed) ||
    !["observed", "fused", "estimated", "unknown"].includes(String(value.quality))
  ) {
    throw new Error("Public replay returned an invalid observation");
  }
  return value as PublicReplayObservation;
}

function interpolateMeasurement<T extends { value: number; unit: string; reference?: string }>(
  previous: T | null,
  next: T | null,
  fraction: number,
): T | null {
  if (!previous) return null;
  if (!next || previous.unit !== next.unit || previous.reference !== next.reference) return previous;
  return { ...previous, value: interpolate(previous.value, next.value, fraction) };
}

function interpolateHeading(previous: number | null, next: number | null, fraction: number) {
  if (previous === null || next === null) return previous;
  const delta = ((next - previous + 540) % 360) - 180;
  return (previous + delta * fraction + 360) % 360;
}

function interpolate(previous: number, next: number, fraction: number) {
  return previous + (next - previous) * fraction;
}

function isAltitude(value: unknown) {
  return value === null || (
    isRecord(value) &&
    typeof value.value === "number" && Number.isFinite(value.value) &&
    (value.unit === "feet" || value.unit === "meters") &&
    isBoundedString(value.reference, 32)
  );
}

function isSpeed(value: unknown) {
  return value === null || (
    isRecord(value) &&
    typeof value.value === "number" && Number.isFinite(value.value) && value.value >= 0 &&
    (value.unit === "knots" || value.unit === "kilometers_per_hour")
  );
}

function isTimestamp(value: unknown): value is string {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function isBoundedString(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.trim().length > 0 && value.length <= maximum;
}

function isIntegerInRange(value: unknown, minimum: number, maximum: number): value is number {
  return Number.isInteger(value) && Number(value) >= minimum && Number(value) <= maximum;
}

function isFiniteInRange(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
