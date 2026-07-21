import { bench, describe } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { AirportObservation, Hazard } from "@/lib/weather-api";
import { OperationsMap, polygonPoints, project } from "./operations-map";

const times = {
  event_time: "2026-07-20T16:00:00Z",
  received_at: "2026-07-20T16:00:05Z",
  processed_at: "2026-07-20T16:00:06Z",
};

const hazards = Array.from({ length: 300 }, (_, index) => hazard(index));
const observations = Array.from({ length: 75 }, (_, index) => observation(index));

describe("representative western-region weather layers", () => {
  bench("projects 300 hazard polygons and 75 METAR stations", () => {
    for (const item of hazards) polygonPoints(item);
    for (const item of observations) {
      project(item.point.longitude_degrees, item.point.latitude_degrees);
    }
  });

  bench("renders the complete 375-item weather layer", () => {
    renderToStaticMarkup(createElement(OperationsMap, {
      flights: [],
      hazards,
      observations,
      sourceHealth: [],
      weatherState: "ready",
      weatherMessage: null,
      weatherAsOf: "2026-07-20T16:05:00Z",
      selectedId: null,
      selectedHazardId: null,
      onSelect: noop,
      onSelectHazard: noop,
      onRetryWeather: noop,
      liveReferenceTime: null,
    }));
  });
});

function hazard(index: number): Hazard {
  const column = index % 20;
  const row = Math.floor(index / 20);
  const longitude = -124.5 + column * 0.5;
  const latitude = 33 + row * 0.8;
  return {
    id: `hazard-${index}`,
    operator_id: "00000000-0000-0000-0000-000000000001",
    schema_version: 1,
    source: source(`hazard-${index}`, "airsigmet"),
    times: times,
    external_series_id: `regional-${index}`,
    revision: 1,
    supersedes_id: null,
    status: "active",
    issued_at: times.event_time,
    provider_received_at: times.received_at,
    hazard_type: "convective",
    severity: index % 7 === 0 ? "severe" : "significant",
    valid_from: "2026-07-20T16:00:00Z",
    valid_to: "2026-07-20T18:00:00Z",
    altitude_band: null,
    footprint: {
      exterior: [
        point(longitude, latitude),
        point(longitude + 0.35, latitude),
        point(longitude + 0.35, latitude + 0.3),
        point(longitude, latitude + 0.3),
        point(longitude, latitude),
      ],
    },
  };
}

function observation(index: number): AirportObservation {
  return {
    id: `observation-${index}`,
    operator_id: "00000000-0000-0000-0000-000000000001",
    schema_version: 1,
    source: source(`observation-${index}`, "metar"),
    times,
    station_code: `K${String(index).padStart(3, "0")}`,
    report_type: "METAR",
    raw_text: "METAR performance fixture",
    provider_received_at: times.received_at,
    point: point(-124 + (index % 15) * 0.6, 33 + Math.floor(index / 15) * 2.8),
    wind_direction_true_degrees: 280,
    wind_speed: { value: 12, unit: "knots" },
    wind_gust: null,
    visibility_statute_miles: 10,
    visibility_greater_than: true,
    ceiling: null,
    flight_category: "visual",
  };
}

function source(envelope_id: string, feed: string) {
  return { envelope_id, provider: "noaa-awc", feed, provider_record_id: envelope_id };
}

function point(longitude_degrees: number, latitude_degrees: number) {
  return { longitude_degrees, latitude_degrees };
}

function noop() {}
