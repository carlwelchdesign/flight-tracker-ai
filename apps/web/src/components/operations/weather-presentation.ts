import type { AirportObservation, Hazard, WeatherSourceHealth } from "@/lib/weather-api";

export type HazardDisplayState = "active" | "upcoming" | "expired" | "cancelled";

export function hazardDisplayState(hazard: Hazard, referenceTime: number): HazardDisplayState {
  if (hazard.status === "cancelled") return "cancelled";
  if (Date.parse(hazard.valid_from) > referenceTime) return "upcoming";
  if (Date.parse(hazard.valid_to) < referenceTime) return "expired";
  return "active";
}

export function hazardStateLabel(state: HazardDisplayState): string {
  return {
    active: "Active",
    upcoming: "Upcoming",
    expired: "Expired",
    cancelled: "Cancelled",
  }[state];
}

export function formatAltitudeBand(hazard: Hazard): string {
  const { altitude_band: band } = hazard;
  if (!band?.lower && !band?.upper) return "Altitude not specified";
  const lower = band.lower ? formatAltitude(band.lower.value, band.lower.unit) : "Surface";
  const upper = band.upper ? formatAltitude(band.upper.value, band.upper.unit) : "Unbounded";
  return `${lower} – ${upper}`;
}

export function observationAgeState(
  observation: AirportObservation,
  referenceTime: number,
): "current" | "stale" {
  return referenceTime - Date.parse(observation.times.event_time) > 15 * 60_000
    ? "stale"
    : "current";
}

export function weatherSourceState(
  health: WeatherSourceHealth[],
): "current" | "stale" | "degraded" | "unknown" {
  if (health.some((item) => ["degraded", "unavailable"].includes(item.state))) return "degraded";
  if (health.some((item) => item.state === "stale")) return "stale";
  if (health.length > 0 && health.every((item) => item.state === "healthy")) return "current";
  return "unknown";
}

export function flightCategoryLabel(category: AirportObservation["flight_category"]): string {
  return {
    visual: "VFR",
    marginal_visual: "MVFR",
    instrument: "IFR",
    low_instrument: "LIFR",
    unknown: "Unknown",
  }[category];
}

function formatAltitude(value: number, unit: "feet" | "meters"): string {
  if (unit === "feet" && value >= 100) return `FL${Math.round(value / 100)}`;
  return `${value.toLocaleString("en-US")} ${unit === "feet" ? "ft" : "m"}`;
}
