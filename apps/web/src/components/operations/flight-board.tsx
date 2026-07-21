import type { FlightView } from "@/lib/fleet-api";
import type { Hazard } from "@/lib/weather-api";
import {
  attentionLevel,
  callsign,
  fleetReferenceTime,
  freshness,
  phaseLabel,
  routeLabel,
  scheduleVariance,
  sourceLabel,
  sourceQualityLabel,
} from "./operations-model";

type FlightBoardProps = {
  flights: FlightView[];
  hazards: Hazard[];
  selectedId: string | null;
  refreshing: boolean;
  controlsAvailable: boolean;
  liveReferenceTime: number | null;
  onSelect: (flightId: string) => void;
  onStart: () => void;
};

export function FlightBoard({
  flights,
  hazards,
  selectedId,
  refreshing,
  controlsAvailable,
  liveReferenceTime,
  onSelect,
  onStart,
}: FlightBoardProps) {
  const referenceTime = fleetReferenceTime(flights);

  return (
    <section className="ops-panel ops-board-panel" aria-labelledby="board-title">
      <div className="ops-panel-heading">
        <div>
          <p className="ops-eyebrow">Operational queue</p>
          <h2 id="board-title">Flight board</h2>
        </div>
        <span className="panel-count" aria-live="polite">
          {refreshing ? "Updating" : `${flights.length} flights`}
        </span>
      </div>

      {flights.length === 0 ? (
        <div className="ops-empty-state">
          <div className="empty-radar" aria-hidden="true"><span /></div>
          <h3>No active flight picture</h3>
          <p>
            The API is connected, but no normalized flights have reached the current-state
            projection yet.
          </p>
          {controlsAvailable ? (
            <button type="button" className="ops-primary-button" onClick={onStart}>
              Start simulation
            </button>
          ) : (
            <p className="empty-hint">Replay controls are unavailable in this environment.</p>
          )}
        </div>
      ) : (
        <div className="flight-table-wrap">
          <table className="flight-table">
            <caption className="sr-only">
              Current flights. Selecting a callsign updates the map and detail panel.
            </caption>
            <thead>
              <tr>
                <th scope="col">Flight</th>
                <th scope="col">Phase</th>
                <th scope="col">Source</th>
                <th scope="col">Variance</th>
                <th scope="col">Freshness</th>
                <th scope="col"><span className="sr-only">Attention</span></th>
              </tr>
            </thead>
            <tbody>
              {flights.map((view) => {
                const selected = view.flight.id === selectedId;
                const variance = scheduleVariance(view);
                const freshnessState = freshness(view, referenceTime, liveReferenceTime);
                const attention = attentionLevel(view, hazards, referenceTime, liveReferenceTime);
                return (
                  <tr key={view.flight.id} data-selected={selected || undefined}>
                    <td>
                      <button
                        type="button"
                        className="flight-select-button"
                        aria-pressed={selected}
                        aria-label={`Select flight ${callsign(view)}, ${routeLabel(view)}`}
                        onClick={() => onSelect(view.flight.id)}
                      >
                        <strong>{callsign(view)}</strong>
                        <span>{routeLabel(view)}</span>
                      </button>
                    </td>
                    <td><span className={`phase phase-${view.flight.status}`}>{phaseLabel(view)}</span></td>
                    <td>
                      <span className="flight-source-label">{sourceLabel(view)}</span>
                      <small className="flight-quality-label">{sourceQualityLabel(view)}</small>
                    </td>
                    <td className={variance.minutes && variance.minutes >= 15 ? "variance-watch" : ""}>
                      {variance.label}
                    </td>
                    <td>
                      <span className={`freshness freshness-${freshnessState.level}`}>
                        {freshnessState.label}
                      </span>
                    </td>
                    <td>
                      <span
                        className={`attention-dot attention-${attention.level}`}
                        title={`${attention.label}: ${attention.reason}`}
                      >
                        <span className="sr-only">
                          {attention.label} attention: {attention.reason}
                        </span>
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
