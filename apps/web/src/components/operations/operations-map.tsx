import type { CSSProperties } from "react";
import type { FlightView, Hazard } from "@/lib/fleet-api";
import {
  AIRPORTS,
  airportFor,
  attentionLevel,
  callsign,
  fleetReferenceTime,
} from "./operations-model";

type OperationsMapProps = {
  flights: FlightView[];
  hazards: Hazard[];
  selectedId: string | null;
  onSelect: (flightId: string) => void;
};

const BOUNDS = {
  west: -125,
  east: -114,
  south: 32.5,
  north: 49,
};

export function OperationsMap({
  flights,
  hazards,
  selectedId,
  onSelect,
}: OperationsMapProps) {
  const referenceTime = fleetReferenceTime(flights);
  const markers = layoutAircraftMarkers(flights);
  const airportCodes = new Set(
    flights.flatMap((view) => [
      view.flight.origin_airport_code,
      view.flight.destination_airport_code,
    ]),
  );

  return (
    <section className="ops-panel ops-map-panel" aria-labelledby="map-title">
      <div className="ops-panel-heading">
        <div>
          <p className="ops-eyebrow">Live geography</p>
          <h2 id="map-title">Fleet map</h2>
        </div>
        <div className="ops-map-legend" aria-label="Map legend">
          <span><i className="legend-aircraft" /> Aircraft</span>
          <span><i className="legend-hazard" /> Weather</span>
        </div>
      </div>

      <div
        className="ops-map"
        role="group"
        aria-label="Western United States route map. Select an aircraft to update the board and detail panel."
      >
        <svg viewBox="0 0 1000 650" aria-hidden="true" className="ops-map-canvas">
          <defs>
            <pattern id="grid" width="80" height="80" patternUnits="userSpaceOnUse">
              <path d="M 80 0 L 0 0 0 80" fill="none" className="map-grid-line" />
            </pattern>
            <linearGradient id="map-glow" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stopColor="#0c2531" />
              <stop offset="1" stopColor="#07141c" />
            </linearGradient>
          </defs>
          <rect width="1000" height="650" fill="url(#map-glow)" />
          <rect width="1000" height="650" fill="url(#grid)" />
          <path
            className="map-coast"
            d="M82 0 C96 55 88 112 126 166 C145 198 125 236 146 282 C164 320 145 358 184 398 C220 436 188 473 238 520 C270 550 278 604 300 650"
          />

          {hazards.map((hazard) => (
            <polygon
              key={hazard.id}
              className={`map-hazard map-hazard-${hazard.severity}`}
              points={hazard.footprint.exterior
                .map((point) => {
                  const projected = project(
                    point.longitude_degrees,
                    point.latitude_degrees,
                  );
                  return `${projected.x * 10},${projected.y * 6.5}`;
                })
                .join(" ")}
            />
          ))}

          {flights.map((view) => {
            const origin = airportFor(view.flight.origin_airport_code);
            const destination = airportFor(view.flight.destination_airport_code);
            if (!origin || !destination) return null;
            const start = project(origin.longitude, origin.latitude);
            const end = project(destination.longitude, destination.latitude);
            const selected = view.flight.id === selectedId;
            return (
              <line
                key={view.flight.id}
                x1={start.x * 10}
                y1={start.y * 6.5}
                x2={end.x * 10}
                y2={end.y * 6.5}
                className={selected ? "map-route map-route-selected" : "map-route"}
              />
            );
          })}

          {Object.values(AIRPORTS)
            .filter((airport) => airportCodes.has(airport.code))
            .map((airport) => {
              const point = project(airport.longitude, airport.latitude);
              return (
                <g key={airport.code} transform={`translate(${point.x * 10} ${point.y * 6.5})`}>
                  <circle r="5" className="map-airport-dot" />
                  <text x="10" y="4" className="map-airport-label">
                    {airport.code}
                  </text>
                </g>
              );
            })}
        </svg>

        {markers.map(({ view, point, offsetX, offsetY }) => {
          const attention = attentionLevel(view, hazards, referenceTime);
          const selected = view.flight.id === selectedId;
          const style = {
            left: `${point.x}%`,
            top: `${point.y}%`,
            "--aircraft-heading": `${view.latest_position?.heading_true_degrees ?? 0}deg`,
            "--aircraft-offset-x": `${offsetX}px`,
            "--aircraft-offset-y": `${offsetY}px`,
          } as CSSProperties;
          return (
            <button
              key={view.flight.id}
              type="button"
              className={`aircraft-marker aircraft-${attention.level} ${
                selected ? "aircraft-selected" : ""
              }`}
              style={style}
              aria-label={`Select ${callsign(view)}, ${attention.label.toLowerCase()} attention`}
              aria-pressed={selected}
              onClick={() => onSelect(view.flight.id)}
            >
              <PlaneGlyph />
              <span>{callsign(view)}</span>
            </button>
          );
        })}

        <div className="map-scale" aria-hidden="true">
          <span />
          200 NM
        </div>
      </div>
    </section>
  );
}

function project(longitude: number, latitude: number) {
  return {
    x: ((longitude - BOUNDS.west) / (BOUNDS.east - BOUNDS.west)) * 100,
    y: ((BOUNDS.north - latitude) / (BOUNDS.north - BOUNDS.south)) * 100,
  };
}

function layoutAircraftMarkers(flights: FlightView[]) {
  const offsets = [
    { x: 0, y: 0 },
    { x: 0, y: -38 },
    { x: 0, y: 38 },
    { x: -52, y: 0 },
    { x: 52, y: 0 },
  ];
  const placed: Array<{ x: number; y: number }> = [];

  return flights.flatMap((view) => {
    const position = view.latest_position?.point;
    const fallback = airportFor(view.flight.origin_airport_code);
    if (!position && !fallback) return [];

    const point = project(
      position?.longitude_degrees ?? fallback!.longitude,
      position?.latitude_degrees ?? fallback!.latitude,
    );
    const collisionIndex = placed.filter(
      (candidate) => Math.abs(candidate.x - point.x) < 6 && Math.abs(candidate.y - point.y) < 6,
    ).length;
    placed.push(point);
    const offset = offsets[collisionIndex % offsets.length];
    return [{ view, point, offsetX: offset.x, offsetY: offset.y }];
  });
}

function PlaneGlyph() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.2c.7 0 1.2.6 1.2 1.3v5.3l7.1 4.2v1.8l-7.1-2.1v4.8l2.1 1.5v1.5L12 19.7l-3.3.8V19l2.1-1.5v-4.8l-7.1 2.1V13l7.1-4.2V3.5c0-.7.5-1.3 1.2-1.3Z" />
    </svg>
  );
}
