import { getApiBaseUrl } from "./fleet-api";

export type LivePositionState =
  | "disabled"
  | "connecting"
  | "current"
  | "degraded"
  | "unavailable";

export type LivePositionStatus = {
  enabled: boolean;
  provider: string | null;
  feed: string | null;
  state: LivePositionState;
  best_effort: boolean;
  observed_at: string;
  last_attempt_at: string | null;
  last_success_at: string | null;
  newest_position_at: string | null;
  consecutive_failures: number;
  aircraft_count: number;
  fresh_position_count: number;
  stale_position_count: number;
  rejected_record_count: number;
  missing_callsign_count: number;
  stale_after_seconds: number;
  last_error_code: string | null;
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

export type LivePositionLoadResult =
  | { state: "ready"; status: LivePositionStatus }
  | { state: "unavailable"; message: string };

export async function getInitialLivePositionStatus(
  assertion: string,
): Promise<LivePositionLoadResult> {
  try {
    const response = await fetch(`${getApiBaseUrl()}/api/live-positions/status`, {
      headers: { authorization: `Bearer ${assertion}` },
      cache: "no-store",
      signal: AbortSignal.timeout(2_500),
    });
    if (!response.ok) {
      return {
        state: "unavailable",
        message: `Live position status returned HTTP ${response.status}`,
      };
    }
    return { state: "ready", status: parseLivePositionStatus(await response.json()) };
  } catch (error) {
    return {
      state: "unavailable",
      message: error instanceof Error ? error.message : "Live position status is unavailable",
    };
  }
}

export function parseLivePositionStatus(value: unknown): LivePositionStatus {
  if (
    !isRecord(value) ||
    typeof value.enabled !== "boolean" ||
    !isOptionalString(value.provider) ||
    !isOptionalString(value.feed) ||
    !["disabled", "connecting", "current", "degraded", "unavailable"].includes(
      String(value.state),
    ) ||
    typeof value.best_effort !== "boolean" ||
    typeof value.observed_at !== "string" ||
    !isOptionalString(value.last_attempt_at) ||
    !isOptionalString(value.last_success_at) ||
    !isOptionalString(value.newest_position_at) ||
    !isNonNegativeNumber(value.consecutive_failures) ||
    !isNonNegativeNumber(value.aircraft_count) ||
    !isNonNegativeNumber(value.fresh_position_count) ||
    !isNonNegativeNumber(value.stale_position_count) ||
    !isNonNegativeNumber(value.rejected_record_count) ||
    !isNonNegativeNumber(value.missing_callsign_count) ||
    !isNonNegativeNumber(value.stale_after_seconds) ||
    !isOptionalString(value.last_error_code) ||
    (value.region !== null && !isRegion(value.region)) ||
    (value.attribution !== null && !isAttribution(value.attribution))
  ) {
    throw new Error("Live position API returned an unexpected status payload");
  }
  return value as LivePositionStatus;
}

function isRegion(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.latitude_degrees === "number" &&
    Number.isFinite(value.latitude_degrees) &&
    typeof value.longitude_degrees === "number" &&
    Number.isFinite(value.longitude_degrees) &&
    isNonNegativeNumber(value.radius_nautical_miles)
  );
}

function isAttribution(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.text === "string" &&
    typeof value.source_url === "string" &&
    typeof value.license_url === "string"
  );
}

function isNonNegativeNumber(value: unknown): boolean {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isOptionalString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
