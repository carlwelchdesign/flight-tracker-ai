export type PublicWeatherState = "current" | "degraded" | "stale" | "unavailable";
export type PublicWeatherSeverity = "advisory" | "significant" | "severe" | "unknown";
export type PublicFlightCategory = "visual" | "marginal_visual" | "instrument" | "low_instrument" | "unknown";

export type PublicWeatherSource = {
  provider: string;
  feed: string;
};

export type PublicWeatherSourceStatus = PublicWeatherSource & {
  state: "healthy" | "degraded" | "stale" | "unavailable" | "unknown";
  observed_at: string;
  last_success_at: string | null;
  newest_event_at: string | null;
  stale_after_seconds: number;
  last_error_code: string | null;
};

export type PublicWeatherPoint = {
  longitude_degrees: number;
  latitude_degrees: number;
};

export type PublicWeatherAltitude = {
  value: number;
  unit: "feet" | "meters";
  reference: string;
};

export type PublicWeatherHazard = {
  id: string;
  source: PublicWeatherSource;
  status: "active" | "cancelled";
  issued_at: string;
  hazard_type: string;
  severity: PublicWeatherSeverity;
  valid_from: string;
  valid_to: string;
  altitude_band: { lower: PublicWeatherAltitude | null; upper: PublicWeatherAltitude | null } | null;
  footprint: { exterior: PublicWeatherPoint[] };
};

export type PublicWeatherObservation = {
  id: string;
  source: PublicWeatherSource;
  observed_at: string;
  received_at: string;
  station_code: string;
  report_type: string;
  point: PublicWeatherPoint;
  wind_direction_true_degrees: number | null;
  wind_speed: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  wind_gust: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  visibility_statute_miles: number | null;
  visibility_greater_than: boolean;
  ceiling: PublicWeatherAltitude | null;
  flight_category: PublicFlightCategory;
};

export type PublicWeatherSnapshot = {
  state: PublicWeatherState;
  generated_at: string;
  attribution: { text: string; source_url: string };
  sources: PublicWeatherSourceStatus[];
  hazards: PublicWeatherHazard[];
  observations: PublicWeatherObservation[];
};

const MAX_HAZARDS = 500;
const MAX_OBSERVATIONS = 200;

export function parsePublicWeatherSnapshot(value: unknown): PublicWeatherSnapshot {
  if (
    !isRecord(value) ||
    !isOneOf(value.state, ["current", "degraded", "stale", "unavailable"]) ||
    !isTimestamp(value.generated_at) ||
    !isAttribution(value.attribution) ||
    !Array.isArray(value.sources) ||
    !Array.isArray(value.hazards) ||
    !Array.isArray(value.observations) ||
    value.hazards.length > MAX_HAZARDS ||
    value.observations.length > MAX_OBSERVATIONS
  ) {
    throw new Error("Public weather returned an unexpected payload");
  }
  return {
    state: value.state,
    generated_at: value.generated_at,
    attribution: value.attribution,
    sources: value.sources.map(parseSourceStatus),
    hazards: value.hazards.map(parseHazard),
    observations: value.observations.map(parseObservation),
  };
}

function parseSourceStatus(value: unknown): PublicWeatherSourceStatus {
  if (!isRecord(value)) throw new Error("Public weather returned an invalid source status");
  const record = value as Record<string, unknown>;
  if (
    !isSource(record) ||
    !isOneOf(record.state, ["healthy", "degraded", "stale", "unavailable", "unknown"]) ||
    !isTimestamp(record.observed_at) ||
    !isOptionalTimestamp(record.last_success_at) ||
    !isOptionalTimestamp(record.newest_event_at) ||
    !isNonNegativeNumber(record.stale_after_seconds) ||
    !isOptionalString(record.last_error_code)
  ) throw new Error("Public weather returned an invalid source status");
  return record as PublicWeatherSourceStatus;
}

function parseHazard(value: unknown): PublicWeatherHazard {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !isSource(value.source) ||
    !isOneOf(value.status, ["active", "cancelled"]) ||
    !isTimestamp(value.issued_at) ||
    typeof value.hazard_type !== "string" ||
    !isOneOf(value.severity, ["advisory", "significant", "severe", "unknown"]) ||
    !isTimestamp(value.valid_from) ||
    !isTimestamp(value.valid_to) ||
    Date.parse(value.valid_from) > Date.parse(value.valid_to) ||
    (value.altitude_band !== null && !isAltitudeBand(value.altitude_band)) ||
    !isRecord(value.footprint) ||
    !Array.isArray(value.footprint.exterior) ||
    !isClosedPolygon(value.footprint.exterior)
  ) throw new Error("Public weather returned an invalid hazard");
  return value as PublicWeatherHazard;
}

function parseObservation(value: unknown): PublicWeatherObservation {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !isSource(value.source) ||
    !isTimestamp(value.observed_at) ||
    !isTimestamp(value.received_at) ||
    typeof value.station_code !== "string" ||
    typeof value.report_type !== "string" ||
    !isPoint(value.point) ||
    !isOptionalHeading(value.wind_direction_true_degrees) ||
    !isOptionalMeasurement(value.wind_speed) ||
    !isOptionalMeasurement(value.wind_gust) ||
    !isOptionalNonNegativeNumber(value.visibility_statute_miles) ||
    typeof value.visibility_greater_than !== "boolean" ||
    (value.ceiling !== null && !isAltitude(value.ceiling)) ||
    !isOneOf(value.flight_category, ["visual", "marginal_visual", "instrument", "low_instrument", "unknown"])
  ) throw new Error("Public weather returned an invalid airport observation");
  return value as PublicWeatherObservation;
}

function isAttribution(value: unknown): value is PublicWeatherSnapshot["attribution"] {
  return isRecord(value) && typeof value.text === "string" && typeof value.source_url === "string";
}

function isSource(value: unknown): boolean {
  return isRecord(value) && typeof value.provider === "string" && typeof value.feed === "string";
}

function isClosedPolygon(value: unknown[]): boolean {
  if (value.length < 4 || !value.every(isPoint)) return false;
  const first = value[0] as PublicWeatherPoint;
  const last = value.at(-1) as PublicWeatherPoint;
  return first.longitude_degrees === last.longitude_degrees && first.latitude_degrees === last.latitude_degrees;
}

function isPoint(value: unknown): value is PublicWeatherPoint {
  return isRecord(value) && finiteRange(value.longitude_degrees, -180, 180) && finiteRange(value.latitude_degrees, -90, 90);
}

function isAltitude(value: unknown): value is PublicWeatherAltitude {
  return isRecord(value) && Number.isFinite(value.value) &&
    isOneOf(value.unit, ["feet", "meters"]) && typeof value.reference === "string";
}

function isAltitudeBand(value: unknown): boolean {
  return isRecord(value) && (value.lower === null || isAltitude(value.lower)) &&
    (value.upper === null || isAltitude(value.upper));
}

function isOptionalMeasurement(value: unknown): boolean {
  return value === null || (isRecord(value) && isNonNegativeNumber(value.value) &&
    isOneOf(value.unit, ["knots", "kilometers_per_hour"]));
}

function isTimestamp(value: unknown): value is string {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function isOptionalTimestamp(value: unknown): boolean {
  return value === null || isTimestamp(value);
}

function isOptionalHeading(value: unknown): boolean {
  return value === null || finiteRange(value, 0, 360, false);
}

function isOptionalString(value: unknown): boolean {
  return value === null || typeof value === "string";
}

function isOptionalNonNegativeNumber(value: unknown): boolean {
  return value === null || isNonNegativeNumber(value);
}

function isNonNegativeNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function finiteRange(value: unknown, minimum: number, maximum: number, inclusiveMaximum = true): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum &&
    (inclusiveMaximum ? value <= maximum : value < maximum);
}

function isOneOf<T extends string>(value: unknown, values: readonly T[]): value is T {
  return typeof value === "string" && values.includes(value as T);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
