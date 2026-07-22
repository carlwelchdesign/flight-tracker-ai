import type {
  PublicWeatherHazard,
  PublicWeatherObservation,
  PublicWeatherSnapshot,
  PublicWeatherState,
} from "@/lib/public-weather";
import { hazardLifecycle, selectedWeather, type WeatherSelection } from "./public-weather-map";
import { WIND_LEVELS, type PublicWindField, type WindLevelCode } from "@/lib/public-atmosphere";

export type AtmosphericLayerControlModel = {
  showRadar: boolean;
  showSatellite: boolean;
  showSurfaceWind: boolean;
  showModelWind: boolean;
  windLevel: WindLevelCode;
  windState: "idle" | "loading" | "current" | "degraded" | "unavailable";
  windField: PublicWindField | null;
  onShowRadar: (value: boolean) => void;
  onShowSatellite: (value: boolean) => void;
  onShowSurfaceWind: (value: boolean) => void;
  onShowModelWind: (value: boolean) => void;
  onWindLevel: (value: WindLevelCode) => void;
};

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
  atmosphere?: AtmosphericLayerControlModel;
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
  atmosphere,
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

      {atmosphere && <AtmosphericControls model={atmosphere} />}

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

function AtmosphericControls({ model }: { model: AtmosphericLayerControlModel }) {
  return (
    <section className="atmospheric-controls" aria-label="Atmospheric map layers">
      <fieldset className="public-weather-toggles atmospheric-toggles">
        <legend>Atmospheric layers</legend>
        <label>
          <input type="checkbox" checked={model.showRadar} onChange={(event) => model.onShowRadar(event.target.checked)} />
          Radar
        </label>
        <label>
          <input type="checkbox" checked={model.showSatellite} onChange={(event) => model.onShowSatellite(event.target.checked)} />
          Satellite clouds
        </label>
        <label>
          <input type="checkbox" checked={model.showSurfaceWind} onChange={(event) => model.onShowSurfaceWind(event.target.checked)} />
          Surface wind barbs
        </label>
        <label>
          <input type="checkbox" checked={model.showModelWind} onChange={(event) => model.onShowModelWind(event.target.checked)} />
          Animated winds
        </label>
      </fieldset>
      {model.showModelWind && (
        <label className="atmospheric-level-control">
          Model wind level
          <select
            aria-label="Model wind level"
            value={model.windLevel}
            onChange={(event) => model.onWindLevel(event.target.value as WindLevelCode)}
          >
            {WIND_LEVELS.map((level) => <option key={level.code} value={level.code}>{level.label}</option>)}
          </select>
        </label>
      )}
      <div className={`atmospheric-source atmospheric-source-${model.windState}`} role="status">
        <strong>{windStateLabel(model)}</strong>
        {model.windField && (
          <span>
            {formatTimestamp(model.windField.forecast_time)} · {meanWindSpeed(model.windField)} kt mean
          </span>
        )}
      </div>
      <div className="atmospheric-attribution">
        <a href="https://nowcoast.noaa.gov/" target="_blank" rel="noreferrer">NOAA nowCOAST imagery</a>
        {model.windField && (
          <>
            <a href={model.windField.attribution.source_url} target="_blank" rel="noreferrer">{model.windField.attribution.text}</a>
            <a href={model.windField.attribution.license_url} target="_blank" rel="noreferrer">CC BY 4.0</a>
          </>
        )}
        <small>Advisory portfolio context · not for navigation or flight briefing</small>
      </div>
    </section>
  );
}

function windStateLabel(model: AtmosphericLayerControlModel) {
  if (!model.showModelWind) return "Model wind hidden";
  if (model.windState === "loading") return "Loading model wind";
  if (model.windState === "unavailable") return "Model wind unavailable";
  if (model.windState === "degraded") return "Last model wind field";
  if (model.windState === "current") return model.windField?.level.label ?? "Current model wind";
  return "Model wind idle";
}

function meanWindSpeed(field: PublicWindField) {
  return Math.round(field.samples.reduce((sum, sample) => sum + sample.speed_knots, 0) / field.samples.length);
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
