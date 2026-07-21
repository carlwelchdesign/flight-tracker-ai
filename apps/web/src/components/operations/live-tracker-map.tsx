"use client";

import { useEffect, useRef, useState } from "react";
import type { Map as MapLibreMap, Marker as MapLibreMarker } from "maplibre-gl";
import type { PublicAircraft, PublicLiveStatus } from "@/lib/public-live-positions";

type Props = {
  aircraft: PublicAircraft[];
  selectedId: string | null;
  status: PublicLiveStatus | null;
  mode: "live" | "stale" | "replay";
  onSelect: (id: string) => void;
};

type MarkerEntry = { marker: MapLibreMarker; element: HTMLButtonElement; animationFrame: number | null };

const DEFAULT_CENTER: [number, number] = [-122.38, 37.62];

export function LiveTrackerMap({ aircraft, selectedId, status, mode, onSelect }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<MapLibreMap | null>(null);
  const markersRef = useRef(new Map<string, MarkerEntry>());
  const onSelectRef = useRef(onSelect);
  const [mapReady, setMapReady] = useState(false);
  const [mapError, setMapError] = useState<string | null>(null);
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
          if (!disposed) setMapReady(true);
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
          `${item.heading_true_degrees ?? 0}deg`,
        );
        animateMarker(entry, [item.longitude_degrees, item.latitude_degrees]);
      }
      if (!hasFitRef.current && aircraft.length > 0) {
        hasFitRef.current = true;
        fitTraffic(map, aircraft);
      }
    });
  }, [aircraft, mapReady, selectedId, status]);

  function handleFitTraffic() {
    if (mapRef.current) fitTraffic(mapRef.current, aircraft);
  }

  return (
    <section className="ops-panel live-map-panel" aria-labelledby="live-map-title">
      <div className="ops-panel-heading live-map-heading">
        <div>
          <p className="ops-eyebrow">Navigable airspace</p>
          <h2 id="live-map-title">Bay Area traffic</h2>
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
      </div>
    </section>
  );
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
