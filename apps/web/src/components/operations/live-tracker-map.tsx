"use client";

import { useEffect, useRef, useState } from "react";
import type { FeatureCollection, LineString, Point } from "geojson";
import type { GeoJSONSource, Map as MapLibreMap, Marker as MapLibreMarker } from "maplibre-gl";
import type { EstimatedTrajectory, TrajectoryPoint } from "@/lib/flight-trajectories";
import type { PublicAircraft, PublicLiveStatus } from "@/lib/public-live-positions";
import type { PublicLiveRegion } from "@/lib/public-live-regions";
import type { PublicWeatherSnapshot, PublicWeatherState } from "@/lib/public-weather";
import { liveMarkerRotationDegrees } from "./aircraft-marker-heading";
import { PublicWeatherOverlay } from "./public-weather-overlay";
import { weatherGeoJson, type WeatherSelection } from "./public-weather-map";

type Props = {
  aircraft: PublicAircraft[];
  region: PublicLiveRegion;
  selectedId: string | null;
  status: PublicLiveStatus | null;
  mode: "live" | "stale" | "replay";
  trail: readonly TrajectoryPoint[];
  projection: EstimatedTrajectory | null;
  weather: PublicWeatherSnapshot | null;
  weatherState: PublicWeatherState | "loading";
  weatherRetained: boolean;
  onRetryWeather: () => void;
  onSelect: (id: string) => void;
};

type MarkerEntry = { marker: MapLibreMarker; element: HTMLButtonElement; animationFrame: number | null };

const DEFAULT_CENTER: [number, number] = [-122.38, 37.62];
const TRAJECTORY_SOURCE_ID = "selected-aircraft-trajectory";
const OBSERVED_TRAIL_LAYER_ID = "selected-aircraft-observed-trail";
const PROJECTED_TRAJECTORY_LAYER_ID = "selected-aircraft-projected-trajectory";
const PROJECTED_ENDPOINT_LAYER_ID = "selected-aircraft-projected-endpoint";
const WEATHER_SOURCE_ID = "public-noaa-weather";
const WEATHER_HAZARD_FILL_LAYER_ID = "public-weather-hazard-fill";
const WEATHER_HAZARD_LINE_LAYER_ID = "public-weather-hazard-line";
const WEATHER_OBSERVATION_LAYER_ID = "public-weather-observation";
const WEATHER_STATION_LABEL_LAYER_ID = "public-weather-station-label";

export function LiveTrackerMap({
  aircraft,
  region,
  selectedId,
  status,
  mode,
  trail,
  projection,
  weather,
  weatherState,
  weatherRetained,
  onRetryWeather,
  onSelect,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<MapLibreMap | null>(null);
  const markersRef = useRef(new Map<string, MarkerEntry>());
  const onSelectRef = useRef(onSelect);
  const [mapReady, setMapReady] = useState(false);
  const [mapError, setMapError] = useState<string | null>(null);
  const [showHazards, setShowHazards] = useState(true);
  const [showObservations, setShowObservations] = useState(true);
  const [weatherSelection, setWeatherSelection] = useState<WeatherSelection | null>(null);
  const hasFitRef = useRef(false);

  useEffect(() => {
    onSelectRef.current = onSelect;
  }, [onSelect]);

  useEffect(() => {
    if (!containerRef.current || mapRef.current) return;
    let disposed = false;
    let resizeObserver: ResizeObserver | null = null;
    const markerEntries = markersRef.current;
    void import("maplibre-gl").then((maplibre) => {
      if (disposed || !containerRef.current) return;
      try {
        const map = new maplibre.Map({
          container: containerRef.current,
          style: "https://tiles.openfreemap.org/styles/dark",
          center: DEFAULT_CENTER,
          zoom: 7.5,
          pitch: 18,
          attributionControl: { compact: true },
        });
        map.addControl(new maplibre.NavigationControl({ visualizePitch: true }), "top-right");
        map.addControl(new maplibre.FullscreenControl(), "top-right");
        map.on("load", () => {
          if (disposed) return;
          addWeatherLayers(map, setWeatherSelection);
          addTrajectoryLayers(map);
          setMapReady(true);
        });
        map.on("error", (event) => {
          if (!disposed && event.error) setMapError("Basemap temporarily unavailable");
        });
        resizeObserver = new ResizeObserver(() => map.resize());
        resizeObserver.observe(containerRef.current);
        mapRef.current = map;
      } catch {
        if (!disposed) setMapError("This browser could not initialize the interactive map");
      }
    });
    return () => {
      disposed = true;
      resizeObserver?.disconnect();
      for (const entry of markerEntries.values()) {
        if (entry.animationFrame) cancelAnimationFrame(entry.animationFrame);
        entry.marker.remove();
      }
      markerEntries.clear();
      mapRef.current?.remove();
      mapRef.current = null;
    };
  }, []);

  useEffect(() => {
    const map = mapRef.current;
    hasFitRef.current = false;
    if (!map || !mapReady) return;
    map.easeTo({ center: [...region.center], zoom: 7.5, duration: 600 });
  }, [mapReady, region]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    const visibleIds = new Set(aircraft.map((item) => item.id));
    for (const [id, entry] of markersRef.current) {
      if (!visibleIds.has(id)) {
        if (entry.animationFrame) cancelAnimationFrame(entry.animationFrame);
        entry.marker.remove();
        markersRef.current.delete(id);
      }
    }
    void import("maplibre-gl").then(({ Marker }) => {
      for (const item of aircraft) {
        let entry = markersRef.current.get(item.id);
        if (!entry) {
          const element = document.createElement("button");
          element.type = "button";
          element.className = "live-aircraft-marker";
          element.dataset.aircraftId = item.id;
          element.innerHTML = '<span aria-hidden="true">✈</span>';
          element.addEventListener("click", () => onSelectRef.current(item.id));
          const marker = new Marker({ element, anchor: "center" })
            .setLngLat([item.longitude_degrees, item.latitude_degrees])
            .addTo(map);
          entry = { marker, element, animationFrame: null };
          markersRef.current.set(item.id, entry);
        }
        entry.element.classList.toggle("is-selected", item.id === selectedId);
        entry.element.classList.toggle("is-stale", isStale(item, status));
        entry.element.setAttribute(
          "aria-label",
          `Select ${displayCallsign(item)}, observed ${formatTime(item.observed_at)}`,
        );
        entry.element.style.setProperty(
          "--aircraft-heading",
          `${liveMarkerRotationDegrees(item.heading_true_degrees)}deg`,
        );
        animateMarker(entry, [item.longitude_degrees, item.latitude_degrees]);
      }
      if (!hasFitRef.current && aircraft.length > 0) {
        hasFitRef.current = true;
        fitTraffic(map, aircraft);
      }
    });
  }, [aircraft, mapReady, selectedId, status]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    const source = map.getSource(TRAJECTORY_SOURCE_ID) as GeoJSONSource | undefined;
    source?.setData(trajectoryGeoJson(mode === "replay" ? [] : trail, mode === "replay" ? null : projection));
  }, [mapReady, mode, projection, trail]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    const source = map.getSource(WEATHER_SOURCE_ID) as GeoJSONSource | undefined;
    source?.setData(weatherGeoJson(weather, showHazards, showObservations, weatherSelection));
  }, [mapReady, showHazards, showObservations, weather, weatherSelection]);

  function handleShowHazards(value: boolean) {
    setShowHazards(value);
    if (!value && weatherSelection?.kind === "hazard") setWeatherSelection(null);
  }

  function handleShowObservations(value: boolean) {
    setShowObservations(value);
    if (!value && weatherSelection?.kind === "observation") setWeatherSelection(null);
  }

  function handleFitTraffic() {
    if (mapRef.current) fitTraffic(mapRef.current, aircraft);
  }

  return (
    <section className="ops-panel live-map-panel" aria-labelledby="live-map-title">
      <div className="ops-panel-heading live-map-heading">
        <div>
          <p className="ops-eyebrow">Navigable airspace</p>
          <h2 id="live-map-title">{region.name} traffic</h2>
        </div>
        <div className="live-map-actions">
          <span className={`live-source-pill source-${mode}`}>
            <i aria-hidden="true" /> {mode === "live" ? "Live ADS-B" : mode === "stale" ? "Last live picture" : "Replay"}
          </span>
          <button type="button" onClick={handleFitTraffic} disabled={aircraft.length === 0}>
            Fit traffic
          </button>
        </div>
      </div>
      <div className="live-map-stage">
        <div
          ref={containerRef}
          className="maplibre-canvas"
          aria-label="Interactive aircraft map. Drag to pan, scroll to zoom, and use the controls to rotate or reset north."
        />
        {!mapReady && <div className="map-loading">Loading navigable map…</div>}
        {mapError && <div className="map-error" role="status">{mapError}</div>}
        <div className="map-help">Drag to pan · Scroll to zoom · Right-drag to rotate</div>
        <PublicWeatherOverlay
          snapshot={weather}
          state={weatherState}
          retained={weatherRetained}
          showHazards={showHazards}
          showObservations={showObservations}
          selection={weatherSelection}
          onShowHazards={handleShowHazards}
          onShowObservations={handleShowObservations}
          onSelect={setWeatherSelection}
          onRetry={onRetryWeather}
        />
        {mode !== "replay" && (
          <aside className="trajectory-legend" aria-label="Selected aircraft trajectory legend">
            <span className="trajectory-observed"><i aria-hidden="true" />Observed trail</span>
            <small>{trail.length < 2 ? "Starts after next refresh" : `${trail.length} source points`}</small>
            <span className="trajectory-estimated"><i aria-hidden="true" />Estimated 5-min projection</span>
            <small>{projection ? `${projection.distance_nautical_miles.toFixed(1)} NM at current motion` : "Heading or speed unavailable"}</small>
          </aside>
        )}
      </div>
    </section>
  );
}

function addWeatherLayers(map: MapLibreMap, onSelect: (selection: WeatherSelection) => void) {
  map.addSource(WEATHER_SOURCE_ID, {
    type: "geojson",
    data: weatherGeoJson(null, true, true, null),
  });
  map.addLayer({
    id: WEATHER_HAZARD_FILL_LAYER_ID,
    type: "fill",
    source: WEATHER_SOURCE_ID,
    filter: ["==", ["get", "kind"], "hazard"],
    paint: {
      "fill-color": [
        "match", ["get", "severity"],
        "severe", "#f05b55",
        "significant", "#f2a65a",
        "advisory", "#d6c56a",
        "#80969d",
      ],
      "fill-opacity": [
        "case",
        ["==", ["get", "lifecycle"], "active"], 0.24,
        ["==", ["get", "lifecycle"], "upcoming"], 0.14,
        0.07,
      ],
    },
  });
  map.addLayer({
    id: WEATHER_HAZARD_LINE_LAYER_ID,
    type: "line",
    source: WEATHER_SOURCE_ID,
    filter: ["==", ["get", "kind"], "hazard"],
    paint: {
      "line-color": [
        "match", ["get", "severity"],
        "severe", "#ff7770",
        "significant", "#f2a65a",
        "advisory", "#d6c56a",
        "#80969d",
      ],
      "line-width": ["case", ["boolean", ["get", "selected"], false], 4, 2],
      "line-opacity": ["case", ["==", ["get", "lifecycle"], "active"], 0.95, 0.45],
    },
  });
  map.addLayer({
    id: WEATHER_OBSERVATION_LAYER_ID,
    type: "circle",
    source: WEATHER_SOURCE_ID,
    filter: ["==", ["get", "kind"], "observation"],
    paint: {
      "circle-radius": ["case", ["boolean", ["get", "selected"], false], 9, 7],
      "circle-color": [
        "match", ["get", "flight_category"],
        "visual", "#62d98a",
        "marginal_visual", "#6bb7ff",
        "instrument", "#f05b55",
        "low_instrument", "#d47cff",
        "#9aaeb4",
      ],
      "circle-stroke-color": "#061117",
      "circle-stroke-width": ["case", ["boolean", ["get", "selected"], false], 4, 2],
    },
  });
  map.addLayer({
    id: WEATHER_STATION_LABEL_LAYER_ID,
    type: "symbol",
    source: WEATHER_SOURCE_ID,
    filter: ["==", ["get", "kind"], "observation"],
    layout: {
      "text-field": ["get", "station_code"],
      "text-size": 10,
      "text-offset": [0, 1.45],
      "text-anchor": "top",
      "text-allow-overlap": false,
    },
    paint: {
      "text-color": "#dce8ea",
      "text-halo-color": "#061117",
      "text-halo-width": 2,
    },
  });
  map.on("click", WEATHER_HAZARD_FILL_LAYER_ID, (event) => {
    const id = event.features?.[0]?.properties?.id;
    if (typeof id === "string") onSelect({ kind: "hazard", id });
  });
  map.on("click", WEATHER_OBSERVATION_LAYER_ID, (event) => {
    const id = event.features?.[0]?.properties?.id;
    if (typeof id === "string") onSelect({ kind: "observation", id });
  });
  for (const layer of [WEATHER_HAZARD_FILL_LAYER_ID, WEATHER_OBSERVATION_LAYER_ID]) {
    map.on("mouseenter", layer, () => { map.getCanvas().style.cursor = "pointer"; });
    map.on("mouseleave", layer, () => { map.getCanvas().style.cursor = ""; });
  }
}

function animateMarker(entry: MarkerEntry, destination: [number, number]) {
  if (entry.animationFrame) cancelAnimationFrame(entry.animationFrame);
  const start = entry.marker.getLngLat();
  if (reducedMotion() || (start.lng === destination[0] && start.lat === destination[1])) {
    entry.marker.setLngLat(destination);
    entry.animationFrame = null;
    return;
  }
  const startedAt = performance.now();
  const duration = 1_400;
  const frame = (now: number) => {
    const progress = Math.min(1, (now - startedAt) / duration);
    const eased = 1 - Math.pow(1 - progress, 3);
    entry.marker.setLngLat([
      start.lng + (destination[0] - start.lng) * eased,
      start.lat + (destination[1] - start.lat) * eased,
    ]);
    if (progress < 1) entry.animationFrame = requestAnimationFrame(frame);
    else entry.animationFrame = null;
  };
  entry.animationFrame = requestAnimationFrame(frame);
}

function fitTraffic(map: MapLibreMap, aircraft: PublicAircraft[]) {
  if (aircraft.length === 0) return;
  let west = aircraft[0].longitude_degrees;
  let east = west;
  let south = aircraft[0].latitude_degrees;
  let north = south;
  for (const item of aircraft.slice(1)) {
    west = Math.min(west, item.longitude_degrees);
    east = Math.max(east, item.longitude_degrees);
    south = Math.min(south, item.latitude_degrees);
    north = Math.max(north, item.latitude_degrees);
  }
  if (west === east && south === north) {
    map.easeTo({ center: [west, south], zoom: 10, duration: reducedMotion() ? 0 : 700 });
    return;
  }
  map.fitBounds([[west, south], [east, north]], {
    padding: 70,
    maxZoom: 11,
    duration: reducedMotion() ? 0 : 700,
  });
}

function reducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function addTrajectoryLayers(map: MapLibreMap) {
  map.addSource(TRAJECTORY_SOURCE_ID, {
    type: "geojson",
    lineMetrics: true,
    data: trajectoryGeoJson([], null),
  });
  map.addLayer({
    id: OBSERVED_TRAIL_LAYER_ID,
    type: "line",
    source: TRAJECTORY_SOURCE_ID,
    filter: ["==", ["get", "kind"], "observed"],
    layout: { "line-cap": "round", "line-join": "round" },
    paint: {
      "line-width": 4,
      "line-gradient": [
        "interpolate", ["linear"], ["line-progress"],
        0, "rgba(92, 225, 185, 0.12)",
        1, "rgba(92, 225, 185, 0.95)",
      ],
    },
  });
  map.addLayer({
    id: PROJECTED_TRAJECTORY_LAYER_ID,
    type: "line",
    source: TRAJECTORY_SOURCE_ID,
    filter: ["==", ["get", "kind"], "estimated"],
    layout: { "line-cap": "round" },
    paint: {
      "line-color": "#f2a65a",
      "line-width": 3,
      "line-opacity": 0.9,
      "line-dasharray": [2, 2],
    },
  });
  map.addLayer({
    id: PROJECTED_ENDPOINT_LAYER_ID,
    type: "circle",
    source: TRAJECTORY_SOURCE_ID,
    filter: ["==", ["get", "kind"], "estimated-endpoint"],
    paint: {
      "circle-radius": 5,
      "circle-color": "#f2a65a",
      "circle-stroke-color": "#07141a",
      "circle-stroke-width": 2,
    },
  });
}

function trajectoryGeoJson(
  trail: readonly TrajectoryPoint[],
  projection: EstimatedTrajectory | null,
): FeatureCollection<LineString | Point, { kind: string }> {
  const features: FeatureCollection<LineString | Point, { kind: string }>["features"] = [];
  if (trail.length >= 2) {
    features.push({
      type: "Feature",
      properties: { kind: "observed" },
      geometry: {
        type: "LineString",
        coordinates: trail.map((point) => [point.longitude_degrees, point.latitude_degrees]),
      },
    });
  }
  if (projection) {
    features.push({
      type: "Feature",
      properties: { kind: "estimated" },
      geometry: {
        type: "LineString",
        coordinates: [
          [projection.start.longitude_degrees, projection.start.latitude_degrees],
          [projection.end.longitude_degrees, projection.end.latitude_degrees],
        ],
      },
    });
    features.push({
      type: "Feature",
      properties: { kind: "estimated-endpoint" },
      geometry: {
        type: "Point",
        coordinates: [projection.end.longitude_degrees, projection.end.latitude_degrees],
      },
    });
  }
  return { type: "FeatureCollection", features };
}

function isStale(item: PublicAircraft, status: PublicLiveStatus | null) {
  if (!status) return false;
  return Date.now() - Date.parse(item.observed_at) > status.stale_after_seconds * 1_000;
}

export function displayCallsign(item: PublicAircraft) {
  return item.callsign ?? item.aircraft_registration ?? `Aircraft ${item.id.slice(0, 6)}`;
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone: "UTC",
  }).format(new Date(value)) + "Z";
}
