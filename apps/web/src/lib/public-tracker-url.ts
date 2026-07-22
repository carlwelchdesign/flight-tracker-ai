import { DEFAULT_PUBLIC_LIVE_REGION, findPublicLiveRegion } from "@/lib/public-live-regions";
import type { WindLevelCode } from "@/lib/public-atmosphere";

export const PORTFOLIO_SCENARIO_ID = "m1-operations-v1";
export const DEFAULT_REPLAY_ELAPSED_MS = 60_000;

export type PublicMapView = {
  longitude: number;
  latitude: number;
  zoom: number;
  bearing: number;
  pitch: number;
};

export type PublicWeatherLayers = {
  observations: boolean;
  hazards: boolean;
  radar: boolean;
  satellite: boolean;
  surfaceWind: boolean;
  modelWind: boolean;
  windLevel: WindLevelCode;
};

export type PublicTrackerUrlState = {
  mode: "live" | "replay";
  regionCode: string;
  aircraftKey: string | null;
  replayElapsedMs: number;
  mapView: PublicMapView | null;
  weather: PublicWeatherLayers;
};

export const DEFAULT_PUBLIC_WEATHER_LAYERS: PublicWeatherLayers = {
  observations: true,
  hazards: true,
  radar: true,
  satellite: true,
  surfaceWind: false,
  modelWind: true,
  windLevel: "500",
};

const LAYERS = ["metar", "sigmet", "radar", "satellite", "surface-wind", "model-wind"] as const;
const WIND_LEVELS: readonly WindLevelCode[] = ["surface", "850", "700", "500", "300"];
const AIRCRAFT_KEY = /^[A-Z0-9-]{1,16}$/;

export function defaultPublicTrackerUrlState(): PublicTrackerUrlState {
  return {
    mode: "live",
    regionCode: DEFAULT_PUBLIC_LIVE_REGION.code,
    aircraftKey: null,
    replayElapsedMs: DEFAULT_REPLAY_ELAPSED_MS,
    mapView: null,
    weather: { ...DEFAULT_PUBLIC_WEATHER_LAYERS },
  };
}

export function parsePublicTrackerUrl(search: string): PublicTrackerUrlState {
  const result = defaultPublicTrackerUrlState();
  if (search.length > 1_024) return result;
  const params = new URLSearchParams(search);
  const region = params.get("region");
  if (region && findPublicLiveRegion(region)) result.regionCode = region;

  const scenario = params.get("scenario");
  if (params.get("mode") === "replay" && (!scenario || scenario === PORTFOLIO_SCENARIO_ID)) {
    result.mode = "replay";
  }

  const aircraft = params.get("aircraft")?.trim().toUpperCase() ?? "";
  if (AIRCRAFT_KEY.test(aircraft)) result.aircraftKey = aircraft;

  const elapsed = parseBoundedNumber(params.get("t"), 0, 15 * 60_000);
  if (elapsed !== null) result.replayElapsedMs = Math.round(elapsed);

  const view = params.get("view")?.split(",") ?? [];
  if (view.length === 5) {
    const longitude = parseBoundedNumber(view[0], -180, 180);
    const latitude = parseBoundedNumber(view[1], -85, 85);
    const zoom = parseBoundedNumber(view[2], 2, 14);
    const bearing = parseBoundedNumber(view[3], -180, 180);
    const pitch = parseBoundedNumber(view[4], 0, 60);
    if ([longitude, latitude, zoom, bearing, pitch].every((value) => value !== null)) {
      result.mapView = {
        longitude: longitude!, latitude: latitude!, zoom: zoom!, bearing: bearing!, pitch: pitch!,
      };
    }
  }

  if (params.has("layers")) {
    const enabled = new Set((params.get("layers") ?? "").split(",").filter((layer) => LAYERS.includes(layer as typeof LAYERS[number])));
    result.weather = {
      ...result.weather,
      observations: enabled.has("metar"),
      hazards: enabled.has("sigmet"),
      radar: enabled.has("radar"),
      satellite: enabled.has("satellite"),
      surfaceWind: enabled.has("surface-wind"),
      modelWind: enabled.has("model-wind"),
    };
  }
  const level = params.get("level") as WindLevelCode | null;
  if (level && WIND_LEVELS.includes(level)) result.weather.windLevel = level;
  return result;
}

export function serializePublicTrackerUrl(state: PublicTrackerUrlState): string {
  const params = new URLSearchParams();
  if (state.mode === "replay") {
    params.set("mode", "replay");
    params.set("scenario", PORTFOLIO_SCENARIO_ID);
    params.set("t", String(Math.round(state.replayElapsedMs / 1_000) * 1_000));
  }
  if (findPublicLiveRegion(state.regionCode) && state.regionCode !== DEFAULT_PUBLIC_LIVE_REGION.code) {
    params.set("region", state.regionCode);
  }
  if (state.aircraftKey && AIRCRAFT_KEY.test(state.aircraftKey)) params.set("aircraft", state.aircraftKey);

  const enabled = [
    state.weather.observations && "metar",
    state.weather.hazards && "sigmet",
    state.weather.radar && "radar",
    state.weather.satellite && "satellite",
    state.weather.surfaceWind && "surface-wind",
    state.weather.modelWind && "model-wind",
  ].filter(Boolean).join(",");
  const defaultEnabled = "metar,sigmet,radar,satellite,model-wind";
  if (enabled !== defaultEnabled) params.set("layers", enabled);
  if (state.weather.windLevel !== DEFAULT_PUBLIC_WEATHER_LAYERS.windLevel) params.set("level", state.weather.windLevel);
  if (state.mapView) {
    params.set("view", [
      state.mapView.longitude.toFixed(4),
      state.mapView.latitude.toFixed(4),
      state.mapView.zoom.toFixed(2),
      state.mapView.bearing.toFixed(1),
      state.mapView.pitch.toFixed(1),
    ].join(","));
  }
  const query = params.toString();
  return query ? `?${query}` : "";
}

export function aircraftUrlKey(aircraft: { callsign: string | null; aircraft_registration: string | null; icao_hex?: string | null }): string | null {
  const candidate = aircraft.callsign ?? aircraft.icao_hex ?? aircraft.aircraft_registration;
  if (!candidate) return null;
  const normalized = candidate.trim().toUpperCase();
  return AIRCRAFT_KEY.test(normalized) ? normalized : null;
}

export function aircraftMatchesKey(
  aircraft: { callsign: string | null; aircraft_registration: string | null; icao_hex?: string | null },
  key: string,
): boolean {
  return [aircraft.callsign, aircraft.icao_hex, aircraft.aircraft_registration]
    .some((value) => value?.trim().toUpperCase() === key);
}

function parseBoundedNumber(value: string | null | undefined, minimum: number, maximum: number): number | null {
  if (value == null || value.trim() === "") return null;
  const number = Number(value);
  return Number.isFinite(number) && number >= minimum && number <= maximum ? number : null;
}
