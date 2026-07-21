export type PublicAircraft = {
  id: string;
  callsign: string | null;
  aircraft_registration: string | null;
  longitude_degrees: number;
  latitude_degrees: number;
  altitude: { value: number; unit: "feet" | "meters"; reference: string } | null;
  heading_true_degrees: number | null;
  ground_speed: { value: number; unit: "knots" | "kilometers_per_hour" } | null;
  quality: "observed" | "fused" | "estimated" | "unknown";
  observed_at: string;
  received_at: string;
  provider: string;
};

export type PublicLiveStatus = {
  enabled: boolean;
  provider: string | null;
  state: "disabled" | "connecting" | "current" | "degraded" | "unavailable";
  best_effort: boolean;
  observed_at: string;
  last_success_at: string | null;
  newest_position_at: string | null;
  aircraft_count: number;
  fresh_position_count: number;
  stale_position_count: number;
  stale_after_seconds: number;
  region: {
    latitude_degrees: number;
    longitude_degrees: number;
    radius_nautical_miles: number;
  } | null;
  attribution: {
    text: string;
    source_url: string;
    license_url: string;
  } | null;
};

export type PublicLiveSnapshot = {
  status: PublicLiveStatus;
  data: PublicAircraft[];
};

export function parsePublicLiveSnapshot(value: unknown): PublicLiveSnapshot {
  if (!isRecord(value) || !isStatus(value.status) || !Array.isArray(value.data)) {
    throw new Error("Live tracker returned an unexpected payload");
  }
  return { status: value.status, data: value.data.map(parseAircraft) };
}

function parseAircraft(value: unknown): PublicAircraft {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !nullableString(value.callsign) ||
    !nullableString(value.aircraft_registration) ||
    !finiteCoordinate(value.longitude_degrees, -180, 180) ||
    !finiteCoordinate(value.latitude_degrees, -90, 90) ||
    typeof value.observed_at !== "string" ||
    typeof value.received_at !== "string" ||
    typeof value.provider !== "string"
  ) {
    throw new Error("Live tracker returned an invalid aircraft record");
  }
  return value as PublicAircraft;
}

function isStatus(value: unknown): value is PublicLiveStatus {
  return (
    isRecord(value) &&
    typeof value.enabled === "boolean" &&
    typeof value.state === "string" &&
    ["disabled", "connecting", "current", "degraded", "unavailable"].includes(value.state) &&
    typeof value.observed_at === "string" &&
    typeof value.aircraft_count === "number"
  );
}

function nullableString(value: unknown): boolean {
  return value === null || typeof value === "string";
}

function finiteCoordinate(value: unknown, minimum: number, maximum: number): boolean {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
