export type PublicAirportIntelligence = {
  state: "current" | "partial" | "retained" | "unavailable";
  generated_at: string;
  airport: { code: string; name: string; latitude_degrees: number; longitude_degrees: number };
  attribution: { text: string; source_url: string };
  taf: { state: "current" | "retained" | "unavailable"; accepted_at: string | null; data: PublicTaf | null };
  pireps: { state: "current" | "retained" | "unavailable"; accepted_at: string | null; data: PublicPirep[] | null };
  coverage_note: string;
};

export type PublicTaf = { issue_time: string; valid_from: string; valid_to: string; periods: PublicTafPeriod[] };
export type PublicTafPeriod = { valid_from: string; valid_to: string; change: string; probability_percent: number | null; wind_direction_degrees: number | null; wind_speed_knots: number | null; wind_gust_knots: number | null; visibility: string | null; weather: string | null; clouds: { coverage: string; base_feet_agl: number | null }[] };
export type PublicPirep = { report_time: string; received_at: string; distance_nautical_miles: number; altitude_feet: number | null; altitude_context: string | null; report_type: string; aircraft_type: string | null; turbulence: string | null; icing: string | null; clouds: string | null; wind: { direction_degrees: number; speed_knots: number } | null; temperature_celsius: number | null; weather: string | null; location_available: boolean };

export function parsePublicAirportIntelligence(value: unknown): PublicAirportIntelligence {
  if (!record(value) || !oneOf(value.state, ["current", "partial", "retained", "unavailable"]) ||
    !timestamp(value.generated_at) || !record(value.airport) || typeof value.airport.code !== "string" ||
    typeof value.airport.name !== "string" || !finite(value.airport.latitude_degrees) || !finite(value.airport.longitude_degrees) ||
    !record(value.attribution) || typeof value.attribution.text !== "string" || typeof value.attribution.source_url !== "string" ||
    !record(value.taf) || !record(value.pireps) || typeof value.coverage_note !== "string") {
    throw new Error("Airport intelligence returned an unexpected payload");
  }
  const result = value as unknown as PublicAirportIntelligence;
  if (!feed(result.taf) || !feed(result.pireps) || (result.taf.data && !validTaf(result.taf.data)) ||
    (result.pireps.data && (!Array.isArray(result.pireps.data) || result.pireps.data.length > 20 || !result.pireps.data.every(validPirep)))) {
    throw new Error("Airport intelligence returned invalid forecast or report data");
  }
  return result;
}

function feed(value: { state: string; accepted_at: string | null; data: unknown }) {
  return oneOf(value.state, ["current", "retained", "unavailable"]) && (value.accepted_at === null || timestamp(value.accepted_at));
}
function validTaf(value: PublicTaf) { return timestamp(value.issue_time) && timestamp(value.valid_from) && timestamp(value.valid_to) && Array.isArray(value.periods) && value.periods.length <= 16 && value.periods.every((period) => timestamp(period.valid_from) && timestamp(period.valid_to) && typeof period.change === "string" && Array.isArray(period.clouds)); }
function validPirep(value: PublicPirep) { return timestamp(value.report_time) && timestamp(value.received_at) && finite(value.distance_nautical_miles) && value.distance_nautical_miles >= 0 && value.distance_nautical_miles <= 100 && typeof value.report_type === "string" && typeof value.location_available === "boolean"; }
function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null; }
function timestamp(value: unknown): value is string { return typeof value === "string" && Number.isFinite(Date.parse(value)); }
function finite(value: unknown): value is number { return typeof value === "number" && Number.isFinite(value); }
function oneOf<T extends string>(value: unknown, options: readonly T[]): value is T { return typeof value === "string" && options.includes(value as T); }
