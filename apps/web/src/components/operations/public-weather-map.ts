import type { FeatureCollection, Point, Polygon } from "geojson";
import type {
  PublicWeatherHazard,
  PublicWeatherObservation,
  PublicWeatherSnapshot,
} from "@/lib/public-weather";

export type WeatherSelection = { kind: "hazard" | "observation"; id: string };
export type WeatherFeatureProperties = {
  kind: WeatherSelection["kind"];
  id: string;
  severity?: string;
  lifecycle?: string;
  station_code?: string;
  flight_category?: string;
  selected: boolean;
};

export function weatherGeoJson(
  snapshot: PublicWeatherSnapshot | null,
  showHazards: boolean,
  showObservations: boolean,
  selection: WeatherSelection | null,
  now = Date.now(),
): FeatureCollection<Polygon | Point, WeatherFeatureProperties> {
  if (!snapshot) return { type: "FeatureCollection", features: [] };
  const hazards = showHazards ? snapshot.hazards.map((hazard) => ({
    type: "Feature" as const,
    properties: {
      kind: "hazard" as const,
      id: hazard.id,
      severity: hazard.severity,
      lifecycle: hazardLifecycle(hazard, now),
      selected: selection?.kind === "hazard" && selection.id === hazard.id,
    },
    geometry: {
      type: "Polygon" as const,
      coordinates: [hazard.footprint.exterior.map((point) => [point.longitude_degrees, point.latitude_degrees])],
    },
  })) : [];
  const observations = showObservations ? snapshot.observations.map((observation) => ({
    type: "Feature" as const,
    properties: {
      kind: "observation" as const,
      id: observation.id,
      station_code: observation.station_code,
      flight_category: observation.flight_category,
      selected: selection?.kind === "observation" && selection.id === observation.id,
    },
    geometry: {
      type: "Point" as const,
      coordinates: [observation.point.longitude_degrees, observation.point.latitude_degrees],
    },
  })) : [];
  return { type: "FeatureCollection", features: [...hazards, ...observations] };
}

export function hazardLifecycle(
  hazard: Pick<PublicWeatherHazard, "status" | "valid_from" | "valid_to">,
  now = Date.now(),
): "active" | "upcoming" | "expired" | "cancelled" {
  if (hazard.status === "cancelled") return "cancelled";
  if (now < Date.parse(hazard.valid_from)) return "upcoming";
  if (now > Date.parse(hazard.valid_to)) return "expired";
  return "active";
}

export function selectedWeather(
  snapshot: PublicWeatherSnapshot | null,
  selection: WeatherSelection | null,
): PublicWeatherHazard | PublicWeatherObservation | null {
  if (!snapshot || !selection) return null;
  return selection.kind === "hazard"
    ? snapshot.hazards.find((item) => item.id === selection.id) ?? null
    : snapshot.observations.find((item) => item.id === selection.id) ?? null;
}
