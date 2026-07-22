"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import type { FeatureCollection, LineString, Point } from "geojson";
import type { GeoJSONSource, Map as MapLibreMap, Marker as MapLibreMarker } from "maplibre-gl";
import type { EstimatedTrajectory, TrajectoryPoint } from "@/lib/flight-trajectories";
import type { PublicAircraft, PublicLiveStatus } from "@/lib/public-live-positions";
import type { PublicLiveRegion } from "@/lib/public-live-regions";
import type { PublicWeatherSnapshot, PublicWeatherState } from "@/lib/public-weather";
import type { PublicMapView, PublicWeatherLayers } from "@/lib/public-tracker-url";
import {
  parsePublicWindField,
  type PublicWindField,
  type WindLevelCode,
} from "@/lib/public-atmosphere";
import {
  addAtmosphericRasterLayers,
  setAtmosphericRasterVisibility,
} from "./atmospheric-layers";
import { liveMarkerRotationDegrees } from "./aircraft-marker-heading";
import { PublicWeatherOverlay } from "./public-weather-overlay";
import { selectedWeather, weatherGeoJson, type WeatherSelection } from "./public-weather-map";
import { WindParticleLayer } from "./wind-particle-layer";
import { AirportIntelligencePanel } from "./airport-intelligence-panel";
import { MapFloatingPanel } from "./map-floating-panel";

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
  view: PublicMapView | null;
  layers: PublicWeatherLayers;
  onRetryWeather: () => void;
  onSelect: (id: string) => void;
  onViewChange: (view: PublicMapView) => void;
  onLayersChange: (layers: PublicWeatherLayers) => void;
};

type MarkerEntry = { marker: MapLibreMarker; element: HTMLButtonElement; animationFrame: number | null };
type MapPanelId = "weather" | "airport";

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
  view,
  layers,
  onRetryWeather,
  onSelect,
  onViewChange,
  onLayersChange,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<MapLibreMap | null>(null);
  const markersRef = useRef(new Map<string, MarkerEntry>());
  const windFieldRef = useRef<PublicWindField | null>(null);
  const initialViewRef = useRef(view);
  const onSelectRef = useRef(onSelect);
  const onViewChangeRef = useRef(onViewChange);
  const onWeatherSelectRef = useRef<(selection: WeatherSelection | null) => void>(() => undefined);
  const [mapInstance, setMapInstance] = useState<MapLibreMap | null>(null);
  const [mapReady, setMapReady] = useState(false);
  const [mapError, setMapError] = useState<string | null>(null);
  const [windField, setWindField] = useState<PublicWindField | null>(null);
  const [windState, setWindState] = useState<"idle" | "loading" | "current" | "degraded" | "unavailable">("idle");
  const [weatherSelection, setWeatherSelection] = useState<WeatherSelection | null>(null);
  const [visiblePanels, setVisiblePanels] = useState<Record<MapPanelId, boolean>>({ weather: true, airport: true });
  const [activePanel, setActivePanel] = useState<MapPanelId>("weather");
  const panelMenuSummaryRef = useRef<HTMLElement>(null);
  const hasFitRef = useRef(false);
  const regionalAirport = region.airport === "DEMO" ? "SFO" : region.airport;
  const selectedAirportWeather = selectedWeather(weather, weatherSelection);
  const airportSelected = weatherSelection?.kind === "observation" &&
    selectedAirportWeather !== null && "station_code" in selectedAirportWeather &&
    selectedAirportWeather.station_code === `K${regionalAirport}`;

  const handleWeatherSelection = useCallback((selection: WeatherSelection | null) => {
    setWeatherSelection(selection);
    const selected = selectedWeather(weather, selection);
    const selectsRegionalAirport = selection?.kind === "observation" && selected !== null &&
      "station_code" in selected && selected.station_code === `K${regionalAirport}`;
    if (selectsRegionalAirport) {
      setVisiblePanels((current) => ({ ...current, airport: true }));
      setActivePanel("airport");
    }
  }, [regionalAirport, weather]);

  useEffect(() => {
    onSelectRef.current = onSelect;
  }, [onSelect]);

  useEffect(() => {
    onViewChangeRef.current = onViewChange;
  }, [onViewChange]);

  useEffect(() => {
    onWeatherSelectRef.current = handleWeatherSelection;
  }, [handleWeatherSelection]);

  useEffect(() => {
    windFieldRef.current = windField;
  }, [windField]);

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
          center: initialViewRef.current ? [initialViewRef.current.longitude, initialViewRef.current.latitude] : DEFAULT_CENTER,
          zoom: initialViewRef.current?.zoom ?? 7.5,
          bearing: initialViewRef.current?.bearing ?? 0,
          pitch: initialViewRef.current?.pitch ?? 18,
          attributionControl: { compact: true },
        });
        map.addControl(new maplibre.NavigationControl({ visualizePitch: true }), "top-right");
        map.addControl(new maplibre.FullscreenControl(), "top-right");
        map.on("load", () => {
          if (disposed) return;
          addAtmosphericRasterLayers(map);
          addWeatherLayers(map, (selection) => onWeatherSelectRef.current(selection));
          addTrajectoryLayers(map);
          setMapReady(true);
        });
        map.on("error", (event) => {
          if (disposed || !event.error) return;
          const sourceId = "sourceId" in event && typeof event.sourceId === "string" ? event.sourceId : "";
          setMapError(sourceId.startsWith("noaa-nowcoast")
            ? "A NOAA imagery layer is temporarily unavailable"
            : "Basemap temporarily unavailable");
        });
        map.on("moveend", () => {
          const center = map.getCenter();
          onViewChangeRef.current({
            longitude: center.lng,
            latitude: center.lat,
            zoom: map.getZoom(),
            bearing: map.getBearing(),
            pitch: map.getPitch(),
          });
        });
        resizeObserver = new ResizeObserver(() => map.resize());
        resizeObserver.observe(containerRef.current);
        mapRef.current = map;
        setMapInstance(map);
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
    if (view) {
      if (!viewMatchesMap(map, view)) {
        map.easeTo({ center: [view.longitude, view.latitude], zoom: view.zoom, bearing: view.bearing, pitch: view.pitch, duration: 0 });
      }
      return;
    }
    map.easeTo({ center: [...region.center], zoom: 7.5, bearing: 0, pitch: 18, duration: 600 });
  }, [mapReady, region, view]);

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
      if (!hasFitRef.current && aircraft.length > 0 && !view) {
        hasFitRef.current = true;
        fitTraffic(map, aircraft);
      }
    });
  }, [aircraft, mapReady, selectedId, status, view]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    const source = map.getSource(TRAJECTORY_SOURCE_ID) as GeoJSONSource | undefined;
    source?.setData(trajectoryGeoJson(trail, mode === "replay" ? null : projection));
  }, [mapReady, mode, projection, trail]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    const source = map.getSource(WEATHER_SOURCE_ID) as GeoJSONSource | undefined;
    source?.setData(weatherGeoJson(weather, layers.hazards, layers.observations, weatherSelection));
  }, [layers.hazards, layers.observations, mapReady, weather, weatherSelection]);

  useEffect(() => {
    const map = mapRef.current;
    if (!map || !mapReady) return;
    setAtmosphericRasterVisibility(map, "radar", layers.radar);
    setAtmosphericRasterVisibility(map, "satellite", layers.satellite);
    setAtmosphericRasterVisibility(map, "surfaceWind", layers.surfaceWind);
  }, [layers.radar, layers.satellite, layers.surfaceWind, mapReady]);

  useEffect(() => {
    if (!layers.modelWind) return;
    const controller = new AbortController();
    let disposed = false;

    async function loadWind() {
      if (windFieldRef.current?.region_code !== region.code || windFieldRef.current.level.code !== layers.windLevel) {
        windFieldRef.current = null;
        setWindField(null);
      }
      setWindState("loading");
      try {
        const response = await fetch(`/api/public/atmosphere/wind?region=${region.code}&level=${layers.windLevel}`, {
          cache: "no-store",
          signal: controller.signal,
        });
        if (!response.ok) throw new Error("Atmospheric wind is unavailable");
        const next = parsePublicWindField(await response.json());
        if (next.region_code !== region.code || next.level.code !== layers.windLevel) {
          throw new Error("Atmospheric wind returned the wrong selection");
        }
        if (!disposed) {
          windFieldRef.current = next;
          setWindField(next);
          setWindState(next.state);
        }
      } catch {
        if (disposed || controller.signal.aborted) return;
        const retained = windFieldRef.current?.region_code === region.code
          && windFieldRef.current.level.code === layers.windLevel;
        setWindState(retained ? "degraded" : "unavailable");
        if (!retained) setWindField(null);
      }
    }

    void loadWind();
    const refresh = window.setInterval(loadWind, 15 * 60_000);
    return () => {
      disposed = true;
      window.clearInterval(refresh);
      controller.abort();
    };
  }, [layers.modelWind, layers.windLevel, region.code]);

  function handleShowHazards(value: boolean) {
    onLayersChange({ ...layers, hazards: value });
    if (!value && weatherSelection?.kind === "hazard") setWeatherSelection(null);
  }

  function handleShowObservations(value: boolean) {
    onLayersChange({ ...layers, observations: value });
    if (!value && weatherSelection?.kind === "observation") setWeatherSelection(null);
  }

  function handleFitTraffic() {
    if (mapRef.current) fitTraffic(mapRef.current, aircraft);
  }

  function togglePanel(panel: MapPanelId) {
    const nextVisible = !visiblePanels[panel];
    setVisiblePanels((current) => ({ ...current, [panel]: nextVisible }));
    if (nextVisible) setActivePanel(panel);
  }

  function closePanel(panel: MapPanelId) {
    panelMenuSummaryRef.current?.focus();
    setVisiblePanels((current) => ({ ...current, [panel]: false }));
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
          <details className="map-panel-menu">
            <summary ref={panelMenuSummaryRef}>Panels</summary>
            <div role="group" aria-label="Map panels">
              <button type="button" aria-pressed={visiblePanels.weather} onClick={() => togglePanel("weather")}>
                <span>NOAA layers</span><small>{visiblePanels.weather ? "Shown" : "Hidden"}</small>
              </button>
              <button type="button" aria-pressed={visiblePanels.airport} onClick={() => togglePanel("airport")}>
                <span>{`K${regionalAirport}`} forecast / PIREPs</span><small>{visiblePanels.airport ? "Shown" : "Hidden"}</small>
              </button>
            </div>
          </details>
        </div>
      </div>
      <div className="live-map-stage">
        <div
          ref={containerRef}
          className="maplibre-canvas"
          aria-label="Interactive aircraft map. Drag to pan, scroll to zoom, and use the controls to rotate or reset north."
        />
        <WindParticleLayer map={mapReady ? mapInstance : null} field={windField} visible={layers.modelWind} />
        {!mapReady && <div className="map-loading">Loading navigable map…</div>}
        {mapError && <div className="map-error" role="status">{mapError}</div>}
        <div className="map-help">Drag to pan · Scroll to zoom · Right-drag to rotate</div>
        <MapFloatingPanel
          className="weather-map-panel"
          label="NOAA weather panel"
          title="Weather layers"
          visible={visiblePanels.weather}
          active={activePanel === "weather"}
          onActivate={() => setActivePanel("weather")}
          onClose={() => closePanel("weather")}
        >
          <PublicWeatherOverlay
            snapshot={weather}
            state={weatherState}
            retained={weatherRetained}
            showHazards={layers.hazards}
            showObservations={layers.observations}
            selection={weatherSelection}
            onShowHazards={handleShowHazards}
            onShowObservations={handleShowObservations}
            onSelect={handleWeatherSelection}
            onRetry={onRetryWeather}
            atmosphere={{
              showRadar: layers.radar,
              showSatellite: layers.satellite,
              showSurfaceWind: layers.surfaceWind,
              showModelWind: layers.modelWind,
              windLevel: layers.windLevel,
              windState: layers.modelWind ? windState : "idle",
              windField,
              onShowRadar: (value) => onLayersChange({ ...layers, radar: value }),
              onShowSatellite: (value) => onLayersChange({ ...layers, satellite: value }),
              onShowSurfaceWind: (value) => onLayersChange({ ...layers, surfaceWind: value }),
              onShowModelWind: (value) => onLayersChange({ ...layers, modelWind: value }),
              onWindLevel: (value: WindLevelCode) => onLayersChange({ ...layers, windLevel: value }),
            }}
          />
        </MapFloatingPanel>
        <MapFloatingPanel
          className="airport-map-panel"
          label={`K${regionalAirport} forecast and nearby PIREPs panel`}
          title="Airport intelligence"
          visible={visiblePanels.airport}
          active={activePanel === "airport"}
          onActivate={() => setActivePanel("airport")}
          onClose={() => closePanel("airport")}
        >
          <AirportIntelligencePanel key={region.airport} airport={regionalAirport} forceOpen={airportSelected} />
        </MapFloatingPanel>
        <aside className="trajectory-legend" aria-label="Selected aircraft trajectory legend">
          <span className="trajectory-observed"><i aria-hidden="true" />{mode === "replay" ? "Replay trail" : "Observed trail"}</span>
          <small>{mode === "replay"
            ? trail.length < 2 ? "Waiting for a second scenario point" : `${trail.length} scenario points · current marker may be interpolated`
            : trail.length < 2 ? "Starts after next refresh" : `${trail.length} source points`}</small>
          {mode !== "replay" && (
            <>
              <span className="trajectory-estimated"><i aria-hidden="true" />Estimated 5-min projection</span>
              <small>{projection ? `${projection.distance_nautical_miles.toFixed(1)} NM at current motion` : "Heading or speed unavailable"}</small>
            </>
          )}
        </aside>
      </div>
    </section>
  );
}

function viewMatchesMap(map: MapLibreMap, view: PublicMapView): boolean {
  const center = map.getCenter();
  return Math.abs(center.lng - view.longitude) < 0.0001
    && Math.abs(center.lat - view.latitude) < 0.0001
    && Math.abs(map.getZoom() - view.zoom) < 0.01
    && Math.abs(map.getBearing() - view.bearing) < 0.1
    && Math.abs(map.getPitch() - view.pitch) < 0.1;
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
