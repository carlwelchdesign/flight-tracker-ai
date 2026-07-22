export type PublicAircraft = {
  id: string;
  callsign: string | null;
  aircraft_registration: string | null;
  icao_hex?: string | null;
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
    source_name: string;
    source_url: string;
    terms_label: string;
    terms_url: string;
  } | null;
};

export type PublicLiveSnapshot = {
  region_code: string | null;
  region_name: string | null;
  status: PublicLiveStatus;
  data: PublicAircraft[];
};

export function parsePublicLiveSnapshot(value: unknown): PublicLiveSnapshot {
  if (!isRecord(value) || !isStatus(value.status) || !Array.isArray(value.data)) {
    throw new Error("Live tracker returned an unexpected payload");
  }
  if (!nullableString(value.region_code) || !nullableString(value.region_name)) {
    throw new Error("Live tracker returned an invalid region");
  }
  return {
    region_code: value.region_code,
    region_name: value.region_name,
    status: value.status,
    data: value.data.map(parseAircraft),
  };
}

function parseAircraft(value: unknown): PublicAircraft {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !nullableString(value.callsign) ||
    !nullableString(value.aircraft_registration) ||
    !nullableIcaoHex(value.icao_hex) ||
    !finiteCoordinate(value.longitude_degrees, -180, 180) ||
    !finiteCoordinate(value.latitude_degrees, -90, 90) ||
    typeof value.observed_at !== "string" ||
    typeof value.received_at !== "string" ||
    typeof value.provider !== "string"
  ) {
    throw new Error("Live tracker returned an invalid aircraft record");
  }
  return {
    ...(value as PublicAircraft),
    icao_hex: typeof value.icao_hex === "string" ? value.icao_hex.toUpperCase() : null,
  };
}

function nullableIcaoHex(value: unknown): boolean {
  return value === undefined || value === null
    || (typeof value === "string" && /^[A-Fa-f0-9]{6}$/.test(value));
}

function isStatus(value: unknown): value is PublicLiveStatus {
  return (
    isRecord(value) &&
    typeof value.enabled === "boolean" &&
    typeof value.state === "string" &&
    ["disabled", "connecting", "current", "degraded", "unavailable"].includes(value.state) &&
    typeof value.observed_at === "string" &&
    typeof value.aircraft_count === "number" &&
    nullableString(value.provider) &&
    (value.attribution === null || isAttribution(value.attribution))
  );
}

function isAttribution(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.text === "string" &&
    typeof value.source_name === "string" &&
    typeof value.source_url === "string" &&
    typeof value.terms_label === "string" &&
    typeof value.terms_url === "string"
  );
}

function nullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function finiteCoordinate(value: unknown, minimum: number, maximum: number): boolean {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
