import type { FlightView, Hazard } from "@/lib/fleet-api";

export type AttentionLevel = "normal" | "watch" | "critical";
export type FreshnessLevel = "current" | "aging" | "stale" | "unknown";

export type AirportPoint = {
  code: string;
  name: string;
  longitude: number;
  latitude: number;
};

export const AIRPORTS: Record<string, AirportPoint> = {
  LAX: { code: "LAX", name: "Los Angeles", longitude: -118.4085, latitude: 33.9416 },
  LAS: { code: "LAS", name: "Las Vegas", longitude: -115.1522, latitude: 36.08 },
  SEA: { code: "SEA", name: "Seattle", longitude: -122.3088, latitude: 47.4502 },
  SFO: { code: "SFO", name: "San Francisco", longitude: -122.375, latitude: 37.6188 },
};

export function callsign(view: FlightView): string {
  return view.flight.callsign ?? view.flight.id.slice(0, 8).toUpperCase();
}

export function routeLabel(view: FlightView): string {
  return `${view.flight.origin_airport_code ?? "—"} → ${
    view.flight.destination_airport_code ?? "—"
  }`;
}

export function phaseLabel(view: FlightView): string {
  const labels: Record<FlightView["flight"]["status"], string> = {
    scheduled: "Scheduled",
    active: "En route",
    diverted: "Diverted",
    landed: "Landed",
    cancelled: "Cancelled",
    unknown: "Unknown",
  };
  return labels[view.flight.status];
}

export function scheduleVariance(view: FlightView): { label: string; minutes: number | null } {
  const departure = parseTime(view.flight.scheduled_departure_at);
  const observed = parseTime(view.flight.times.event_time);
  if (departure === null || observed === null) {
    return { label: "Not reported", minutes: null };
  }
  if (view.flight.status !== "scheduled") {
    return { label: "No variance", minutes: 0 };
  }
  const minutes = Math.max(0, Math.round((observed - departure) / 60_000));
  return {
    label: minutes === 0 ? "On schedule" : `+${minutes} min`,
    minutes,
  };
}

export function latestEventTime(view: FlightView): number | null {
  return Math.max(
    parseTime(view.flight.times.event_time) ?? 0,
    parseTime(view.latest_position?.times.event_time ?? null) ?? 0,
  ) || null;
}

export function fleetReferenceTime(flights: FlightView[]): number | null {
  const values = flights.map(latestEventTime).filter((value): value is number => value !== null);
  return values.length > 0 ? Math.max(...values) : null;
}

export function freshness(
  view: FlightView,
  referenceTime: number | null,
): { level: FreshnessLevel; label: string } {
  const eventTime = latestEventTime(view);
  if (eventTime === null || referenceTime === null) {
    return { level: "unknown", label: "No update" };
  }
  const seconds = Math.max(0, Math.round((referenceTime - eventTime) / 1_000));
  if (seconds <= 10) return { level: "current", label: "Current" };
  if (seconds <= 90) return { level: "aging", label: `${seconds}s behind` };
  return { level: "stale", label: `${Math.round(seconds / 60)}m behind` };
}

export function attentionLevel(
  view: FlightView,
  hazards: Hazard[],
  referenceTime: number | null,
): { level: AttentionLevel; label: string; reason: string } {
  const nearbyHazard = hazards.find((hazard) => isPositionNearHazard(view, hazard));
  if (nearbyHazard?.severity === "severe") {
    return { level: "critical", label: "Critical", reason: "Near severe weather" };
  }
  if (nearbyHazard) {
    return { level: "watch", label: "Watch", reason: "Hazard-adjacent track" };
  }
  const variance = scheduleVariance(view);
  if (variance.minutes !== null && variance.minutes >= 15) {
    return { level: "watch", label: "Watch", reason: `${variance.label} departure` };
  }
  const freshnessState = freshness(view, referenceTime);
  if (freshnessState.level === "stale") {
    return { level: "watch", label: "Watch", reason: "Position data is stale" };
  }
  return { level: "normal", label: "Normal", reason: "No active exceptions" };
}

export function formatZulu(value: string | null | undefined): string {
  const timestamp = parseTime(value ?? null);
  if (timestamp === null) return "—";
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone: "UTC",
  }).format(timestamp) + "Z";
}

export function formatAltitude(view: FlightView): string {
  const altitude = view.latest_position?.altitude;
  if (!altitude) return "—";
  const suffix = altitude.unit === "feet" ? "ft" : "m";
  return `${altitude.value.toLocaleString("en-US")} ${suffix}`;
}

export function formatSpeed(view: FlightView): string {
  const speed = view.latest_position?.ground_speed;
  if (!speed) return "—";
  const suffix = speed.unit === "knots" ? "kt" : "km/h";
  return `${Math.round(speed.value)} ${suffix}`;
}

export function airportFor(code: string | null): AirportPoint | null {
  return code ? AIRPORTS[code] ?? null : null;
}

function isPositionNearHazard(view: FlightView, hazard: Hazard): boolean {
  const position = view.latest_position?.point;
  if (!position || hazard.footprint.exterior.length === 0) return false;
  const longitudes = hazard.footprint.exterior.map((point) => point.longitude_degrees);
  const latitudes = hazard.footprint.exterior.map((point) => point.latitude_degrees);
  const margin = 0.22;
  return (
    position.longitude_degrees >= Math.min(...longitudes) - margin &&
    position.longitude_degrees <= Math.max(...longitudes) + margin &&
    position.latitude_degrees >= Math.min(...latitudes) - margin &&
    position.latitude_degrees <= Math.max(...latitudes) + margin
  );
}

function parseTime(value: string | null): number | null {
  if (!value) return null;
  const result = Date.parse(value);
  return Number.isFinite(result) ? result : null;
}
