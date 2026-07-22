import type { PublicAircraft } from "@/lib/public-live-positions";

export const TRAIL_RETENTION_MS = 10 * 60 * 1_000;
export const MAX_TRAIL_POINTS = 25;
export const PROJECTION_HORIZON_MINUTES = 5;

const EARTH_RADIUS_NAUTICAL_MILES = 3_440.065;
const KILOMETERS_PER_NAUTICAL_MILE = 1.852;

export type TrajectoryPoint = {
  longitude_degrees: number;
  latitude_degrees: number;
  observed_at: string;
};

export type TrajectoryHistory = ReadonlyMap<string, readonly TrajectoryPoint[]>;

export type EstimatedTrajectory = {
  start: TrajectoryPoint;
  end: TrajectoryPoint;
  horizon_minutes: number;
  distance_nautical_miles: number;
};

export function updateTrajectoryHistory(
  current: TrajectoryHistory,
  aircraft: readonly PublicAircraft[],
  nowMilliseconds: number,
): Map<string, readonly TrajectoryPoint[]> {
  const cutoff = nowMilliseconds - TRAIL_RETENTION_MS;
  const next = new Map<string, readonly TrajectoryPoint[]>();

  for (const [aircraftId, points] of current) {
    const retained = points.filter((point) => timestamp(point.observed_at) >= cutoff);
    if (retained.length > 0) next.set(aircraftId, retained.slice(-MAX_TRAIL_POINTS));
  }

  for (const item of aircraft) {
    const observedAt = timestamp(item.observed_at);
    if (!Number.isFinite(observedAt) || observedAt < cutoff) continue;

    const existing = next.get(item.id) ?? [];
    const latest = existing.at(-1);
    if (latest && observedAt <= timestamp(latest.observed_at)) continue;

    next.set(item.id, [...existing, toTrajectoryPoint(item)].slice(-MAX_TRAIL_POINTS));
  }

  return next;
}

export function estimateTrajectory(
  aircraft: PublicAircraft,
  horizonMinutes = PROJECTION_HORIZON_MINUTES,
): EstimatedTrajectory | null {
  const speed = speedInKnots(aircraft);
  const heading = aircraft.heading_true_degrees;
  const observedAt = timestamp(aircraft.observed_at);
  if (
    speed === null ||
    speed <= 0 ||
    heading === null ||
    !Number.isFinite(heading) ||
    !Number.isFinite(observedAt) ||
    !Number.isFinite(aircraft.longitude_degrees) ||
    !Number.isFinite(aircraft.latitude_degrees) ||
    !Number.isFinite(horizonMinutes) ||
    horizonMinutes <= 0
  ) {
    return null;
  }

  const distance = speed * (horizonMinutes / 60);
  const angularDistance = distance / EARTH_RADIUS_NAUTICAL_MILES;
  const bearing = degreesToRadians(normalizeHeading(heading));
  const latitude = degreesToRadians(aircraft.latitude_degrees);
  const longitude = degreesToRadians(aircraft.longitude_degrees);
  const destinationLatitude = Math.asin(
    Math.sin(latitude) * Math.cos(angularDistance) +
      Math.cos(latitude) * Math.sin(angularDistance) * Math.cos(bearing),
  );
  const destinationLongitude = longitude + Math.atan2(
    Math.sin(bearing) * Math.sin(angularDistance) * Math.cos(latitude),
    Math.cos(angularDistance) - Math.sin(latitude) * Math.sin(destinationLatitude),
  );
  const estimatedAt = new Date(observedAt + horizonMinutes * 60_000).toISOString();

  return {
    start: toTrajectoryPoint(aircraft),
    end: {
      longitude_degrees: normalizeLongitude(radiansToDegrees(destinationLongitude)),
      latitude_degrees: radiansToDegrees(destinationLatitude),
      observed_at: estimatedAt,
    },
    horizon_minutes: horizonMinutes,
    distance_nautical_miles: distance,
  };
}

function speedInKnots(aircraft: PublicAircraft): number | null {
  const speed = aircraft.ground_speed;
  if (!speed || !Number.isFinite(speed.value)) return null;
  return speed.unit === "knots" ? speed.value : speed.value / KILOMETERS_PER_NAUTICAL_MILE;
}

function toTrajectoryPoint(aircraft: PublicAircraft): TrajectoryPoint {
  return {
    longitude_degrees: aircraft.longitude_degrees,
    latitude_degrees: aircraft.latitude_degrees,
    observed_at: aircraft.observed_at,
  };
}

function timestamp(value: string): number {
  return Date.parse(value);
}

function normalizeHeading(value: number): number {
  return ((value % 360) + 360) % 360;
}

function normalizeLongitude(value: number): number {
  return ((value + 540) % 360) - 180;
}

function degreesToRadians(value: number): number {
  return value * Math.PI / 180;
}

function radiansToDegrees(value: number): number {
  return value * 180 / Math.PI;
}
