import type {
  PublicWeatherHazard,
  PublicWeatherObservation,
  PublicWeatherSnapshot,
  PublicWeatherState,
} from "@/lib/public-weather";
import { hazardLifecycle, selectedWeather, type WeatherSelection } from "./public-weather-map";

type Props = {
  snapshot: PublicWeatherSnapshot | null;
  state: PublicWeatherState | "loading";
  retained: boolean;
  showHazards: boolean;
  showObservations: boolean;
  selection: WeatherSelection | null;
  onShowHazards: (value: boolean) => void;
  onShowObservations: (value: boolean) => void;
  onSelect: (value: WeatherSelection | null) => void;
  onRetry: () => void;
};

export function PublicWeatherOverlay({
  snapshot,
  state,
  retained,
  showHazards,
  showObservations,
  selection,
  onShowHazards,
  onShowObservations,
  onSelect,
  onRetry,
}: Props) {
  const selected = selectedWeather(snapshot, selection);
  const selectionValue = selection ? `${selection.kind}:${selection.id}` : "";
  const sourceTime = latestSourceTime(snapshot);

  function handleSelection(value: string) {
    if (!value) return onSelect(null);
    const separator = value.indexOf(":");
    const kind = value.slice(0, separator);
    const id = value.slice(separator + 1);
    if ((kind === "hazard" || kind === "observation") && id) onSelect({ kind, id });
  }

  return (
    <aside className="public-weather-overlay" aria-label="NOAA aviation weather layers">
      <div className="public-weather-heading">
        <div>
          <span className={`weather-status weather-status-${state}`}><i aria-hidden="true" />{weatherStatusLabel(state, retained)}</span>
          <small>{sourceTime ? `Latest evidence ${formatAge(sourceTime)}` : "No accepted weather evidence"}</small>
        </div>
        {(state === "unavailable" || retained) && <button type="button" onClick={onRetry}>Retry</button>}
      </div>

      <fieldset className="public-weather-toggles">
        <legend>NOAA layers</legend>
        <label>
          <input type="checkbox" checked={showObservations} onChange={(event) => onShowObservations(event.target.checked)} />
          Airports / METAR <strong>{snapshot?.observations.length ?? 0}</strong>
        </label>
        <label>
          <input type="checkbox" checked={showHazards} onChange={(event) => onShowHazards(event.target.checked)} />
          SIGMET hazards <strong>{snapshot?.hazards.length ?? 0}</strong>
        </label>
      </fieldset>

      {(snapshot?.observations.length || snapshot?.hazards.length) ? (
        <label className="weather-evidence-picker">
          Inspect weather evidence
          <select value={selectionValue} onChange={(event) => handleSelection(event.target.value)}>
            <option value="">Choose a station or hazard</option>
            {snapshot.observations.map((observation) => (
              <option key={observation.id} value={`observation:${observation.id}`}>
                {observation.station_code} · {flightCategoryLabel(observation.flight_category)}
              </option>
            ))}
            {snapshot.hazards.map((hazard) => (
              <option key={hazard.id} value={`hazard:${hazard.id}`}>
                {hazard.hazard_type.replaceAll("_", " ")} · {hazard.severity}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      {selected && selection?.kind === "observation" && (
        <ObservationEvidence observation={selected as PublicWeatherObservation} onClose={() => onSelect(null)} />
      )}
      {selected && selection?.kind === "hazard" && (
        <HazardEvidence hazard={selected as PublicWeatherHazard} onClose={() => onSelect(null)} />
      )}

      <a className="weather-attribution" href={snapshot?.attribution.source_url ?? "https://aviationweather.gov/"} target="_blank" rel="noreferrer">
        {snapshot?.attribution.text ?? "NOAA Aviation Weather Center"}
      </a>
    </aside>
  );
}

function ObservationEvidence({ observation, onClose }: { observation: PublicWeatherObservation; onClose: () => void }) {
  return (
    <section className="public-weather-evidence" aria-labelledby="weather-evidence-title">
      <div><span>Airport observation</span><button type="button" onClick={onClose} aria-label="Close weather details">×</button></div>
      <h3 id="weather-evidence-title">{observation.station_code} · {flightCategoryLabel(observation.flight_category)}</h3>
      <dl>
        <WeatherFact label="Observed" value={formatTimestamp(observation.observed_at)} />
        <WeatherFact label="Wind" value={formatWind(observation)} />
        <WeatherFact label="Visibility" value={formatVisibility(observation)} />
        <WeatherFact label="Ceiling" value={formatCeiling(observation)} />
        <WeatherFact label="Source" value={`${providerLabel(observation.source.provider)} · ${observation.source.feed}`} />
        <WeatherFact label="Report" value={observation.report_type} />
      </dl>
    </section>
  );
}

function HazardEvidence({ hazard, onClose }: { hazard: PublicWeatherHazard; onClose: () => void }) {
  return (
    <section className="public-weather-evidence" aria-labelledby="weather-evidence-title">
      <div><span>Selected hazard</span><button type="button" onClick={onClose} aria-label="Close weather details">×</button></div>
      <h3 id="weather-evidence-title">{hazard.hazard_type.replaceAll("_", " ")}</h3>
      <dl>
        <WeatherFact label="State" value={hazardLifecycle(hazard)} />
        <WeatherFact label="Severity" value={hazard.severity} />
        <WeatherFact label="Valid" value={`${formatTimestamp(hazard.valid_from)} – ${formatTimestamp(hazard.valid_to)}`} />
        <WeatherFact label="Altitude" value={formatAltitudeBand(hazard)} />
        <WeatherFact label="Issued" value={formatTimestamp(hazard.issued_at)} />
        <WeatherFact label="Source" value={`${providerLabel(hazard.source.provider)} · ${hazard.source.feed}`} />
      </dl>
    </section>
  );
}

function WeatherFact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function latestSourceTime(snapshot: PublicWeatherSnapshot | null) {
  return snapshot?.sources
    .flatMap((source) => source.newest_event_at ? [source.newest_event_at] : [])
    .sort()
    .at(-1) ?? null;
}

function weatherStatusLabel(state: Props["state"], retained: boolean) {
  if (retained) return "Last NOAA picture";
  return state === "loading" ? "Loading NOAA" : state === "current" ? "Current NOAA" :
    state === "stale" ? "Stale NOAA" : state === "degraded" ? "Degraded NOAA" : "NOAA unavailable";
}

function flightCategoryLabel(value: PublicWeatherObservation["flight_category"]) {
  return ({ visual: "VFR", marginal_visual: "MVFR", instrument: "IFR", low_instrument: "LIFR", unknown: "Unknown" })[value];
}

function formatWind(observation: PublicWeatherObservation) {
  if (!observation.wind_speed) return "Not supplied";
  const direction = observation.wind_direction_true_degrees == null ? "Variable" : `${Math.round(observation.wind_direction_true_degrees)}° true`;
  const unit = observation.wind_speed.unit === "knots" ? "kt" : "km/h";
  const gust = observation.wind_gust ? ` gust ${Math.round(observation.wind_gust.value)} ${unit}` : "";
  return `${direction} · ${Math.round(observation.wind_speed.value)} ${unit}${gust}`;
}

function formatVisibility(observation: PublicWeatherObservation) {
  if (observation.visibility_statute_miles == null) return "Not supplied";
  return `${observation.visibility_greater_than ? ">" : ""}${observation.visibility_statute_miles} sm`;
}

function formatCeiling(observation: PublicWeatherObservation) {
  if (!observation.ceiling) return "No ceiling supplied";
  return `${observation.ceiling.value.toLocaleString()} ${observation.ceiling.unit === "feet" ? "ft" : "m"} AGL`;
}

function formatAltitudeBand(hazard: PublicWeatherHazard) {
  if (!hazard.altitude_band) return "Not supplied";
  const lower = hazard.altitude_band.lower?.value ?? "Surface";
  const upper = hazard.altitude_band.upper?.value ?? "Unbounded";
  return `${lower} – ${upper} ft`;
}

function formatTimestamp(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    month: "short", day: "numeric", hour: "2-digit", minute: "2-digit",
    hour12: false, timeZone: "UTC", timeZoneName: "short",
  }).format(new Date(value));
}

function formatAge(value: string) {
  const minutes = Math.max(0, Math.round((Date.now() - Date.parse(value)) / 60_000));
  return minutes < 1 ? "now" : minutes < 60 ? `${minutes}m ago` : `${Math.floor(minutes / 60)}h ago`;
}

function providerLabel(value: string) {
  return value === "noaa-awc" ? "NOAA AWC" : value;
}
