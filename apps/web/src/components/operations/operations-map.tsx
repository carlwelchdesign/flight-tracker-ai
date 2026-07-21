import { useState, type CSSProperties } from "react";
import type { FlightView } from "@/lib/fleet-api";
import type { AirportObservation, Hazard, WeatherSourceHealth } from "@/lib/weather-api";
import {
  AIRPORTS,
  airportFor,
  attentionLevel,
  callsign,
  fleetReferenceTime,
  formatZulu,
} from "./operations-model";
import {
  flightCategoryLabel,
  formatAltitudeBand,
  hazardDisplayState,
  hazardStateLabel,
  observationAgeState,
  weatherSourceState,
} from "./weather-presentation";

type OperationsMapProps = {
  flights: FlightView[];
  hazards: Hazard[];
  observations: AirportObservation[];
  sourceHealth: WeatherSourceHealth[];
  weatherState: "ready" | "refreshing" | "unavailable";
  weatherMessage: string | null;
  weatherAsOf: string | null;
  selectedId: string | null;
  selectedHazardId: string | null;
  onSelect: (flightId: string) => void;
  onSelectHazard: (hazardId: string | null) => void;
  onRetryWeather: () => void;
};

const BOUNDS = { west: -125, east: -114, south: 32.5, north: 49 };

export function OperationsMap({
  flights,
  hazards,
  observations,
  sourceHealth,
  weatherState,
  weatherMessage,
  weatherAsOf,
  selectedId,
  selectedHazardId,
  onSelect,
  onSelectHazard,
  onRetryWeather,
}: OperationsMapProps) {
  const [showHazards, setShowHazards] = useState(true);
  const [showObservations, setShowObservations] = useState(true);
  const fleetTime = fleetReferenceTime(flights);
  const weatherTime = Date.parse(weatherAsOf ?? "") || fleetTime || 0;
  const selectedHazard = hazards.find((hazard) => hazard.id === selectedHazardId) ?? null;
  const markers = layoutAircraftMarkers(flights);
  const sourceState = weatherSourceState(sourceHealth);
  const airportCodes = new Set(
    flights.flatMap((view) => [
      view.flight.origin_airport_code,
      view.flight.destination_airport_code,
    ]),
  );
  const providerLabels = [...new Set(
    [...hazards, ...observations].map((item) => providerLabel(item.source.provider)),
  )];

  return (
    <section className="ops-panel ops-map-panel" aria-labelledby="map-title">
      <div className="ops-panel-heading">
        <div>
          <p className="ops-eyebrow">Live geography</p>
          <h2 id="map-title">Fleet + weather</h2>
        </div>
        <div className="ops-map-legend" aria-label="Map legend">
          <span><i className="legend-aircraft" /> Aircraft</span>
          <span><i className="legend-hazard" /> Hazard</span>
          <span><i className="legend-observation" /> METAR</span>
        </div>
      </div>

      <div className="weather-layer-bar">
        <fieldset aria-label="Weather map layers">
          <legend className="sr-only">Weather layers</legend>
          <label>
            <input
              type="checkbox"
              checked={showHazards}
              onChange={(event) => {
                setShowHazards(event.target.checked);
                if (!event.target.checked) onSelectHazard(null);
              }}
            />
            Hazards <span>{hazards.length}</span>
          </label>
          <label>
            <input
              type="checkbox"
              checked={showObservations}
              onChange={(event) => setShowObservations(event.target.checked)}
            />
            METARs <span>{observations.length}</span>
          </label>
        </fieldset>
        <div className={`weather-source-summary weather-source-${sourceState}`} role="status">
          <strong>{sourceState === "current" ? "Current" : sourceState}</strong>
          <span>
            {providerLabels.length > 0 ? providerLabels.join(" + ") : "No weather source"}
            {weatherAsOf ? ` · ${formatZulu(weatherAsOf)}` : ""}
          </span>
        </div>
      </div>

      {weatherState === "unavailable" && (
        <div className="weather-partial-banner" role="status">
          <span>Weather refresh unavailable. Retaining the last accepted layer.</span>
          <button type="button" onClick={onRetryWeather}>Retry</button>
          {weatherMessage && <span className="sr-only">{weatherMessage}</span>}
        </div>
      )}

      <div
        className="ops-map"
        role="group"
        aria-label="Western United States route map with weather. Select an aircraft or hazard for details."
      >
        <svg viewBox="0 0 1000 650" aria-hidden="false" className="ops-map-canvas">
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

          {showHazards && hazards.map((hazard) => {
            const displayState = hazardDisplayState(
              hazard,
              weatherReferenceTime(hazard.source.provider, fleetTime, weatherTime),
            );
            const selected = hazard.id === selectedHazardId;
            return (
              <polygon
                key={hazard.id}
                aria-hidden="true"
                className={`map-hazard map-hazard-${hazard.severity} map-hazard-${displayState} ${selected ? "map-hazard-selected" : ""}`}
                points={polygonPoints(hazard)}
              />
            );
          })}

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
                  <text x="10" y="4" className="map-airport-label">{airport.code}</text>
                </g>
              );
            })}
        </svg>

        {showHazards && hazards.map((hazard) => {
          const displayState = hazardDisplayState(
            hazard,
            weatherReferenceTime(hazard.source.provider, fleetTime, weatherTime),
          );
          const centroid = hazardCentroid(hazard);
          const selected = hazard.id === selectedHazardId;
          return (
            <button
              key={`marker-${hazard.id}`}
              type="button"
              className={`hazard-marker hazard-marker-${hazard.severity} hazard-marker-${displayState} ${selected ? "hazard-marker-selected" : ""}`}
              style={{ left: `${centroid.x}%`, top: `${centroid.y}%` }}
              aria-label={`${hazard.hazard_type} hazard, ${hazard.severity}, ${formatAltitudeBand(hazard)}, ${hazardStateLabel(displayState)}, valid ${formatZulu(hazard.valid_from)} to ${formatZulu(hazard.valid_to)}`}
              aria-pressed={selected}
              onClick={() => onSelectHazard(hazard.id)}
            >
              <span aria-hidden="true">!</span>
              {hazard.hazard_type.replaceAll("_", " ")}
            </button>
          );
        })}

        {showObservations && observations.map((observation) => {
          const point = project(
            observation.point.longitude_degrees,
            observation.point.latitude_degrees,
          );
          const ageState = observationAgeState(
            observation,
            weatherReferenceTime(observation.source.provider, fleetTime, weatherTime),
          );
          return (
            <div
              key={observation.id}
              className={`weather-station weather-station-${observation.flight_category} weather-station-${ageState}`}
              style={{ left: `${point.x}%`, top: `${point.y}%` }}
              role="img"
              aria-label={`${observation.station_code} ${flightCategoryLabel(observation.flight_category)}, ${ageState}, observed ${formatZulu(observation.times.event_time)}`}
            >
              <span>{observation.station_code}</span>
              <strong>{flightCategoryLabel(observation.flight_category)}</strong>
            </div>
          );
        })}

        {markers.map(({ view, point, offsetX, offsetY }) => {
          const attention = attentionLevel(view, hazards, fleetTime);
          const selected = view.flight.id === selectedId;
          const style = {
            left: `${point.x}%`, top: `${point.y}%`,
            "--aircraft-heading": `${view.latest_position?.heading_true_degrees ?? 0}deg`,
            "--aircraft-offset-x": `${offsetX}px`,
            "--aircraft-offset-y": `${offsetY}px`,
          } as CSSProperties;
          return (
            <button
              key={view.flight.id}
              type="button"
              className={`aircraft-marker aircraft-${attention.level} ${selected ? "aircraft-selected" : ""}`}
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

        {weatherState === "refreshing" && <span className="weather-refreshing">Updating weather…</span>}
        <div className="map-scale" aria-hidden="true"><span />200 NM</div>
        {selectedHazard && (
          <HazardInspector
            hazard={selectedHazard}
            referenceTime={weatherReferenceTime(
              selectedHazard.source.provider,
              fleetTime,
              weatherTime,
            )}
            onClose={() => onSelectHazard(null)}
          />
        )}
      </div>
    </section>
  );
}

function HazardInspector({
  hazard,
  referenceTime,
  onClose,
}: {
  hazard: Hazard;
  referenceTime: number;
  onClose: () => void;
}) {
  const displayState = hazardDisplayState(hazard, referenceTime);
  return (
    <section className="weather-inspector" aria-labelledby="weather-detail-title">
      <div className="weather-inspector-heading">
        <div>
          <p className="ops-eyebrow">Selected hazard · revision {hazard.revision}</p>
          <h3 id="weather-detail-title">{hazard.hazard_type.replaceAll("_", " ")}</h3>
        </div>
        <button type="button" onClick={onClose} aria-label="Close hazard details">×</button>
      </div>
      <dl>
        <div><dt>State</dt><dd className={`weather-state-${displayState}`}>{hazardStateLabel(displayState)}</dd></div>
        <div><dt>Severity</dt><dd>{hazard.severity}</dd></div>
        <div><dt>Altitude</dt><dd>{formatAltitudeBand(hazard)}</dd></div>
        <div><dt>Valid</dt><dd>{formatZulu(hazard.valid_from)} – {formatZulu(hazard.valid_to)}</dd></div>
        <div><dt>Issued</dt><dd>{formatZulu(hazard.issued_at)}</dd></div>
        <div><dt>Source</dt><dd>{providerLabel(hazard.source.provider)} · {hazard.source.feed}</dd></div>
      </dl>
      {hazard.source.provider === "noaa-awc" ? (
        <a
          href={`/api/backend/api/source-records/${hazard.source.envelope_id}`}
          target="_blank"
          rel="noreferrer"
        >
          Open raw NOAA source <span aria-hidden="true">↗</span>
        </a>
      ) : (
        <p>Replay source · inspect the scenario fixture for raw evidence.</p>
      )}
    </section>
  );
}

export function project(longitude: number, latitude: number) {
  return {
    x: ((longitude - BOUNDS.west) / (BOUNDS.east - BOUNDS.west)) * 100,
    y: ((BOUNDS.north - latitude) / (BOUNDS.north - BOUNDS.south)) * 100,
  };
}

export function polygonPoints(hazard: Hazard): string {
  return hazard.footprint.exterior
    .map((point) => {
      const projected = project(point.longitude_degrees, point.latitude_degrees);
      return `${projected.x * 10},${projected.y * 6.5}`;
    })
    .join(" ");
}

function hazardCentroid(hazard: Hazard): { x: number; y: number } {
  const points = hazard.footprint.exterior;
  const unique = points.length > 1 && points[0].longitude_degrees === points.at(-1)?.longitude_degrees &&
    points[0].latitude_degrees === points.at(-1)?.latitude_degrees
    ? points.slice(0, -1)
    : points;
  const sum = unique.reduce(
    (total, point) => ({
      longitude: total.longitude + point.longitude_degrees,
      latitude: total.latitude + point.latitude_degrees,
    }),
    { longitude: 0, latitude: 0 },
  );
  return project(sum.longitude / unique.length, sum.latitude / unique.length);
}

function layoutAircraftMarkers(flights: FlightView[]) {
  const offsets = [
    { x: 0, y: 0 }, { x: 0, y: -38 }, { x: 0, y: 38 },
    { x: -52, y: 0 }, { x: 52, y: 0 },
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

function providerLabel(provider: string): string {
  if (provider === "noaa-awc") return "NOAA AWC";
  if (provider === "simulation") return "Simulation";
  return provider;
}

function weatherReferenceTime(
  provider: string,
  fleetTime: number | null,
  weatherTime: number,
): number {
  return provider === "simulation" && fleetTime !== null ? fleetTime : weatherTime;
}

function PlaneGlyph() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 2.2c.7 0 1.2.6 1.2 1.3v5.3l7.1 4.2v1.8l-7.1-2.1v4.8l2.1 1.5v1.5L12 19.7l-3.3.8V19l2.1-1.5v-4.8l-7.1 2.1V13l7.1-4.2V3.5c0-.7.5-1.3 1.2-1.3Z" />
    </svg>
  );
}
