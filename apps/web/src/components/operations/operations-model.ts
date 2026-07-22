import type { FlightView } from "@/lib/fleet-api";
import type { Hazard } from "@/lib/weather-api";

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
  return view.flight.callsign ??
    (isLivePosition(view) && view.flight.source.provider_record_id
      ? `ICAO ${view.flight.source.provider_record_id.toUpperCase()}`
      : view.flight.id.slice(0, 8).toUpperCase());
}

export function routeLabel(view: FlightView): string {
  if (isLivePosition(view)) return "Position only · route unavailable";
  const route = `${view.flight.origin_airport_code ?? "—"} → ${
    view.flight.destination_airport_code ?? "—"
  }`;
  return view.flight.source.provider === "simulation" ? `Simulated ${route}` : route;
}

export function phaseLabel(view: FlightView): string {
  if (isLivePosition(view)) return "Position only";
  const labels: Record<FlightView["flight"]["status"], string> = {
    scheduled: "Scheduled",
    active: "En route",
    diverted: "Diverted",
    landed: "Landed",
    cancelled: "Cancelled",
    unknown: "Unknown",
  };
  const label = labels[view.flight.status];
  return view.flight.source.provider === "simulation" ? `Simulated · ${label}` : label;
}

export function scheduleVariance(view: FlightView): { label: string; minutes: number | null } {
  if (isLivePosition(view)) return { label: "Not supplied", minutes: null };
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

export function fleetTiming(flights: FlightView[]): {
  lastEventTime: number | null;
  lastReceivedTime: number | null;
} {
  const eventTimes = flights.flatMap((view) => [
    parseTime(view.flight.times.event_time),
    parseTime(view.latest_position?.times.event_time ?? null),
  ]);
  const receivedTimes = flights.flatMap((view) => [
    parseTime(view.flight.times.received_at),
    parseTime(view.latest_position?.times.received_at ?? null),
  ]);
  return {
    lastEventTime: maximumTime(eventTimes),
    lastReceivedTime: maximumTime(receivedTimes),
  };
}

export function freshness(
  view: FlightView,
  referenceTime: number | null,
  liveReferenceTime: number | null = null,
): { level: FreshnessLevel; label: string } {
  const eventTime = latestEventTime(view);
  const effectiveReference = isLivePosition(view) ? liveReferenceTime : referenceTime;
  if (eventTime === null || effectiveReference === null) {
    return { level: "unknown", label: "No update" };
  }
  const seconds = Math.max(0, Math.round((effectiveReference - eventTime) / 1_000));
  if (seconds <= 10) {
    return { level: "current", label: isLivePosition(view) ? `${seconds}s old` : "Current" };
  }
  if (seconds <= 90) return { level: "aging", label: `${seconds}s behind` };
  return { level: "stale", label: `${Math.round(seconds / 60)}m behind` };
}

export function attentionLevel(
  view: FlightView,
  hazards: Hazard[],
  referenceTime: number | null,
  liveReferenceTime: number | null = null,
): { level: AttentionLevel; label: string; reason: string } {
  const nearbyHazard = hazards.find(
    (hazard) => isHazardActiveAt(hazard, referenceTime) && isPositionNearHazard(view, hazard),
  );
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
  const freshnessState = freshness(view, referenceTime, liveReferenceTime);
  if (freshnessState.level === "stale") {
    return { level: "watch", label: "Watch", reason: "Position data is stale" };
  }
  return { level: "normal", label: "Normal", reason: "No active exceptions" };
}

function isHazardActiveAt(hazard: Hazard, referenceTime: number | null): boolean {
  if (hazard.status !== "active") return false;
  const at = referenceTime ?? Date.parse(hazard.times.event_time);
  return Date.parse(hazard.valid_from) <= at && Date.parse(hazard.valid_to) >= at;
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

export function isLivePosition(view: FlightView): boolean {
  return ["adsb.lol", "airplanes.live"].includes(
    view.latest_position?.source.provider ?? view.flight.source.provider,
  );
}

export function sourceLabel(view: FlightView): string {
  const provider = view.latest_position?.source.provider ?? view.flight.source.provider;
  if (provider === "adsb.lol") return "ADSB.lol · best effort";
  if (provider === "airplanes.live") return "Airplanes.live fallback · best effort";
  if (provider === "simulation") return "Deterministic replay";
  return provider;
}

export function sourceQualityLabel(view: FlightView): string {
  const quality = view.latest_position?.quality;
  const labels = {
    observed: "Observed ADS-B",
    fused: "Fused / MLAT",
    estimated: "Estimated",
    unknown: "Quality unknown",
  } as const;
  return quality ? labels[quality] : "No position quality";
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

function maximumTime(values: Array<number | null>): number | null {
  const valid = values.filter((value): value is number => value !== null);
  return valid.length > 0 ? Math.max(...valid) : null;
}
