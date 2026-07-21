import type { EventTimes, FleetEvent, SourceAttribution } from "./fleet-api";

export type GeoPoint = {
  longitude_degrees: number;
  latitude_degrees: number;
};

export type Altitude = {
  value: number;
  unit: "feet" | "meters";
  reference: string;
};

export type Hazard = {
  id: string;
  operator_id: string;
  schema_version: number;
  source: SourceAttribution;
  times: EventTimes;
  external_series_id: string;
  revision: number;
  supersedes_id: string | null;
  status: "active" | "cancelled";
  issued_at: string;
  provider_received_at: string | null;
  hazard_type: string;
  severity: "advisory" | "significant" | "severe" | "unknown";
  valid_from: string;
  valid_to: string;
  altitude_band: { lower: Altitude | null; upper: Altitude | null } | null;
  footprint: { exterior: GeoPoint[] };
};

export type AirportObservation = {
  id: string;
  operator_id: string;
  schema_version: number;
  source: SourceAttribution;
  times: EventTimes;
  station_code: string;
  report_type: string;
  raw_text: string;
  provider_received_at: string;
  point: GeoPoint;
  wind_direction_true_degrees: number | null;
  wind_speed: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  wind_gust: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  visibility_statute_miles: number | null;
  visibility_greater_than: boolean;
  ceiling: Altitude | null;
  flight_category: "visual" | "marginal_visual" | "instrument" | "low_instrument" | "unknown";
};

export type WeatherSourceHealth = {
  id: string;
  operator_id: string;
  provider: string;
  feed: string;
  state: "healthy" | "degraded" | "stale" | "unavailable" | "unknown";
  observed_at: string;
  last_attempt_at: string;
  last_success_at: string | null;
  newest_event_at: string | null;
  consecutive_failures: number;
  delay_seconds: number | null;
  stale_after_seconds: number;
  last_error_code: string | null;
};

export type WeatherSnapshot = {
  hazards: Hazard[];
  observations: AirportObservation[];
  sourceHealth: WeatherSourceHealth[];
  generatedAt: string;
};

export type WeatherLoadResult =
  | { state: "ready"; snapshot: WeatherSnapshot }
  | { state: "unavailable"; message: string };

type WeatherPage<T> = { data: T[]; generated_at: string };

const DEFAULT_API_BASE_URL = "http://localhost:8080";

export async function getInitialWeather(assertion: string): Promise<WeatherLoadResult> {
  try {
    const [hazards, observations, health] = await Promise.all([
      fetchJson("/api/hazards", assertion),
      fetchJson("/api/airport-observations", assertion),
      fetchJson("/api/source-health", assertion),
    ]);
    return { state: "ready", snapshot: parseWeatherSnapshot(hazards, observations, health) };
  } catch (error) {
    return {
      state: "unavailable",
      message: error instanceof Error ? error.message : "Weather evidence is unavailable",
    };
  }
}

export function parseWeatherSnapshot(
  hazardsValue: unknown,
  observationsValue: unknown,
  healthValue: unknown,
): WeatherSnapshot {
  const hazards = parsePage(hazardsValue, parseHazard, "hazard");
  const observations = parsePage(observationsValue, parseObservation, "observation");
  if (!isRecord(healthValue) || !Array.isArray(healthValue.data)) {
    throw new Error("Weather API returned unexpected source health");
  }
  return {
    hazards: hazards.data,
    observations: observations.data,
    sourceHealth: healthValue.data.map(parseSourceHealth),
    generatedAt: [hazards.generated_at, observations.generated_at].sort().at(-1)!,
  };
}

export function hazardFromEvent(event: FleetEvent): Hazard | null {
  return event.event.event_type === "weather_hazard" ? parseHazardOrNull(event.event.data) : null;
}

export function observationFromEvent(event: FleetEvent): AirportObservation | null {
  return event.event.event_type === "airport_observation"
    ? parseObservationOrNull(event.event.data)
    : null;
}

function parsePage<T>(
  value: unknown,
  parser: (value: unknown) => T,
  label: string,
): WeatherPage<T> {
  if (!isRecord(value) || !Array.isArray(value.data) || typeof value.generated_at !== "string") {
    throw new Error(`Weather API returned an unexpected ${label} page`);
  }
  return { data: value.data.map(parser), generated_at: value.generated_at };
}

function parseHazard(value: unknown): Hazard {
  const parsed = parseHazardOrNull(value);
  if (!parsed) throw new Error("Weather API returned an unexpected hazard");
  return parsed;
}

function parseHazardOrNull(value: unknown): Hazard | null {
  if (
    !isRecord(value) || typeof value.id !== "string" ||
    typeof value.operator_id !== "string" || typeof value.schema_version !== "number" ||
    !isSource(value.source) || !isEventTimes(value.times) ||
    typeof value.external_series_id !== "string" || typeof value.revision !== "number" ||
    !isOptionalString(value.supersedes_id) || !["active", "cancelled"].includes(String(value.status)) ||
    typeof value.issued_at !== "string" || !isOptionalString(value.provider_received_at) ||
    typeof value.hazard_type !== "string" ||
    !["advisory", "significant", "severe", "unknown"].includes(String(value.severity)) ||
    typeof value.valid_from !== "string" || typeof value.valid_to !== "string" ||
    (value.altitude_band !== null && !isAltitudeBand(value.altitude_band)) ||
    !isRecord(value.footprint) || !Array.isArray(value.footprint.exterior) ||
    value.footprint.exterior.length < 4 ||
    !value.footprint.exterior.every(isPoint)
  ) return null;
  return value as Hazard;
}

function parseObservation(value: unknown): AirportObservation {
  const parsed = parseObservationOrNull(value);
  if (!parsed) throw new Error("Weather API returned an unexpected airport observation");
  return parsed;
}

function parseObservationOrNull(value: unknown): AirportObservation | null {
  if (
    !isRecord(value) || typeof value.id !== "string" || typeof value.operator_id !== "string" ||
    typeof value.schema_version !== "number" || !isSource(value.source) || !isEventTimes(value.times) ||
    typeof value.station_code !== "string" || typeof value.report_type !== "string" ||
    typeof value.raw_text !== "string" || typeof value.provider_received_at !== "string" ||
    !isPoint(value.point) || !isOptionalNumber(value.wind_direction_true_degrees) ||
    !isOptionalMeasurement(value.wind_speed) || !isOptionalMeasurement(value.wind_gust) ||
    !isOptionalNumber(value.visibility_statute_miles) ||
    typeof value.visibility_greater_than !== "boolean" ||
    (value.ceiling !== null && !isAltitude(value.ceiling)) ||
    !["visual", "marginal_visual", "instrument", "low_instrument", "unknown"].includes(
      String(value.flight_category),
    )
  ) return null;
  return value as AirportObservation;
}

function parseSourceHealth(value: unknown): WeatherSourceHealth {
  if (
    !isRecord(value) || typeof value.id !== "string" || typeof value.operator_id !== "string" ||
    typeof value.provider !== "string" || typeof value.feed !== "string" ||
    !["healthy", "degraded", "stale", "unavailable", "unknown"].includes(String(value.state)) ||
    typeof value.observed_at !== "string" || typeof value.last_attempt_at !== "string" ||
    !isOptionalString(value.last_success_at) || !isOptionalString(value.newest_event_at) ||
    typeof value.consecutive_failures !== "number" || !isOptionalNumber(value.delay_seconds) ||
    typeof value.stale_after_seconds !== "number" || !isOptionalString(value.last_error_code)
  ) throw new Error("Weather API returned unexpected source health");
  return value as WeatherSourceHealth;
}

async function fetchJson(path: string, assertion: string): Promise<unknown> {
  const response = await fetch(`${apiBaseUrl()}${path}`, {
    headers: { authorization: `Bearer ${assertion}` },
    cache: "no-store",
    signal: AbortSignal.timeout(2_500),
  });
  if (!response.ok) throw new Error(`Weather API returned HTTP ${response.status}`);
  return response.json();
}

function apiBaseUrl(): string {
  return (process.env.API_BASE_URL ?? DEFAULT_API_BASE_URL).replace(/\/$/, "");
}

function isSource(value: unknown): value is SourceAttribution {
  return isRecord(value) && typeof value.envelope_id === "string" &&
    typeof value.provider === "string" && typeof value.feed === "string" &&
    isOptionalString(value.provider_record_id);
}

function isEventTimes(value: unknown): value is EventTimes {
  return isRecord(value) && typeof value.event_time === "string" &&
    typeof value.received_at === "string" && typeof value.processed_at === "string";
}

function isPoint(value: unknown): value is GeoPoint {
  return isRecord(value) && typeof value.longitude_degrees === "number" &&
    typeof value.latitude_degrees === "number";
}

function isAltitude(value: unknown): value is Altitude {
  return isRecord(value) && typeof value.value === "number" &&
    ["feet", "meters"].includes(String(value.unit)) && typeof value.reference === "string";
}

function isAltitudeBand(value: unknown): boolean {
  return isRecord(value) && (value.lower === null || isAltitude(value.lower)) &&
    (value.upper === null || isAltitude(value.upper));
}

function isOptionalMeasurement(value: unknown): boolean {
  return value === null || (isRecord(value) && typeof value.value === "number" &&
    ["knots", "kilometers_per_hour"].includes(String(value.unit)));
}

function isOptionalString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isOptionalNumber(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
