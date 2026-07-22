export const WIND_LEVELS = [
  { code: "surface", label: "Surface · 10 m AGL" },
  { code: "850", label: "850 hPa · ~5,000 ft" },
  { code: "700", label: "700 hPa · ~10,000 ft" },
  { code: "500", label: "500 hPa · ~18,000 ft" },
  { code: "300", label: "300 hPa · ~30,000 ft" },
] as const;

export type WindLevelCode = (typeof WIND_LEVELS)[number]["code"];

export type PublicWindSample = {
  latitude_degrees: number;
  longitude_degrees: number;
  speed_knots: number;
  direction_from_degrees: number;
};

export type PublicWindField = {
  state: "current" | "degraded";
  retained: boolean;
  region_code: string;
  region_name: string;
  level: {
    code: WindLevelCode;
    label: string;
    pressure_hpa: number | null;
    approximate_altitude_feet: number;
  };
  generated_at: string;
  forecast_time: string;
  last_success_at: string;
  last_error_code: string | null;
  attribution: {
    provider: string;
    model: string;
    source_url: string;
    license_url: string;
    text: string;
  };
  samples: PublicWindSample[];
};

export function isWindLevelCode(value: string): value is WindLevelCode {
  return WIND_LEVELS.some((level) => level.code === value);
}

export function parsePublicWindField(value: unknown): PublicWindField {
  if (!isRecord(value) || (value.state !== "current" && value.state !== "degraded")) {
    throw new Error("Atmospheric wind returned an invalid state");
  }
  if (
    typeof value.retained !== "boolean" ||
    !nonempty(value.region_code) ||
    !nonempty(value.region_name) ||
    !isIsoDate(value.generated_at) ||
    !isIsoDate(value.forecast_time) ||
    !isIsoDate(value.last_success_at) ||
    !(value.last_error_code === null || nonempty(value.last_error_code))
  ) {
    throw new Error("Atmospheric wind returned invalid source evidence");
  }
  const level = parseLevel(value.level);
  const attribution = parseAttribution(value.attribution);
  if (!Array.isArray(value.samples) || value.samples.length === 0 || value.samples.length > 25) {
    throw new Error("Atmospheric wind returned an invalid grid");
  }
  const samples = value.samples.map(parseSample);
  return {
    state: value.state,
    retained: value.retained,
    region_code: value.region_code,
    region_name: value.region_name,
    level,
    generated_at: value.generated_at,
    forecast_time: value.forecast_time,
    last_success_at: value.last_success_at,
    last_error_code: value.last_error_code,
    attribution,
    samples,
  };
}

function parseLevel(value: unknown): PublicWindField["level"] {
  if (
    !isRecord(value) ||
    !nonempty(value.code) ||
    !isWindLevelCode(value.code) ||
    !nonempty(value.label) ||
    !(value.pressure_hpa === null || bounded(value.pressure_hpa, 100, 1_000)) ||
    !bounded(value.approximate_altitude_feet, 0, 60_000)
  ) {
    throw new Error("Atmospheric wind returned an invalid level");
  }
  return {
    code: value.code,
    label: value.label,
    pressure_hpa: value.pressure_hpa,
    approximate_altitude_feet: value.approximate_altitude_feet,
  };
}

function parseAttribution(value: unknown): PublicWindField["attribution"] {
  if (
    !isRecord(value) ||
    !nonempty(value.provider) ||
    !nonempty(value.model) ||
    !nonempty(value.text) ||
    !isHttpsUrl(value.source_url) ||
    !isHttpsUrl(value.license_url)
  ) {
    throw new Error("Atmospheric wind returned invalid attribution");
  }
  return {
    provider: value.provider,
    model: value.model,
    source_url: value.source_url,
    license_url: value.license_url,
    text: value.text,
  };
}

function parseSample(value: unknown): PublicWindSample {
  if (
    !isRecord(value) ||
    !bounded(value.latitude_degrees, -90, 90) ||
    !bounded(value.longitude_degrees, -180, 180) ||
    !bounded(value.speed_knots, 0, 250) ||
    !bounded(value.direction_from_degrees, 0, 360)
  ) {
    throw new Error("Atmospheric wind returned an invalid sample");
  }
  return {
    latitude_degrees: value.latitude_degrees,
    longitude_degrees: value.longitude_degrees,
    speed_knots: value.speed_knots,
    direction_from_degrees: value.direction_from_degrees,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function nonempty(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function bounded(value: unknown, min: number, max: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= min && value <= max;
}

function isIsoDate(value: unknown): value is string {
  return nonempty(value) && Number.isFinite(Date.parse(value));
}

function isHttpsUrl(value: unknown): value is string {
  if (!nonempty(value)) return false;
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}
