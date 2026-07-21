import type { FleetEvent, FlightView } from "@/lib/fleet-api";
import type { Hazard } from "@/lib/weather-api";
import {
  attentionLevel,
  callsign,
  fleetReferenceTime,
  formatAltitude,
  formatSpeed,
  formatZulu,
  phaseLabel,
  routeLabel,
  scheduleVariance,
} from "./operations-model";

type FlightDetailProps = {
  selected: FlightView | null;
  flights: FlightView[];
  hazards: Hazard[];
  timeline: FleetEvent[];
  timelineState: "idle" | "loading" | "ready" | "error";
};

export function FlightDetail({
  selected,
  flights,
  hazards,
  timeline,
  timelineState,
}: FlightDetailProps) {
  if (!selected) {
    return (
      <aside className="ops-panel ops-detail-panel" aria-labelledby="detail-title">
        <div className="ops-panel-heading">
          <div>
            <p className="ops-eyebrow">Evidence and context</p>
            <h2 id="detail-title">Flight detail</h2>
          </div>
        </div>
        <div className="detail-empty">
          <span aria-hidden="true">↖</span>
          <p>Select an aircraft or callsign to inspect its operational evidence.</p>
        </div>
      </aside>
    );
  }

  const referenceTime = fleetReferenceTime(flights);
  const attention = attentionLevel(selected, hazards, referenceTime);
  const variance = scheduleVariance(selected);
  const position = selected.latest_position;

  return (
    <aside className="ops-panel ops-detail-panel" aria-labelledby="detail-title">
      <div className="detail-header">
        <div>
          <p className="ops-eyebrow">Selected flight</p>
          <h2 id="detail-title">{callsign(selected)}</h2>
          <p className="detail-route">{routeLabel(selected)}</p>
        </div>
        <span className={`attention-badge attention-badge-${attention.level}`}>
          {attention.label}
        </span>
      </div>

      <div className={`attention-summary attention-summary-${attention.level}`}>
        <span className={`attention-dot attention-${attention.level}`} aria-hidden="true" />
        <div>
          <strong>{attention.reason}</strong>
          <p>Advisory display only · verify source data before operational action.</p>
        </div>
      </div>

      <dl className="detail-metrics">
        <div>
          <dt>Phase</dt>
          <dd>{phaseLabel(selected)}</dd>
        </div>
        <div>
          <dt>Schedule variance</dt>
          <dd className={variance.minutes && variance.minutes >= 15 ? "variance-watch" : ""}>
            {variance.label}
          </dd>
        </div>
        <div>
          <dt>Altitude</dt>
          <dd>{formatAltitude(selected)}</dd>
        </div>
        <div>
          <dt>Ground speed</dt>
          <dd>{formatSpeed(selected)}</dd>
        </div>
        <div>
          <dt>Heading</dt>
          <dd>
            {position?.heading_true_degrees === null || position?.heading_true_degrees === undefined
              ? "—"
              : `${Math.round(position.heading_true_degrees)}°T`}
          </dd>
        </div>
        <div>
          <dt>Last event</dt>
          <dd>{formatZulu(position?.times.event_time ?? selected.flight.times.event_time)}</dd>
        </div>
      </dl>

      <section className="detail-section" aria-labelledby="schedule-title">
        <div className="detail-section-title">
          <h3 id="schedule-title">Schedule</h3>
          <span>{selected.flight.aircraft_registration ?? "Tail unknown"}</span>
        </div>
        <dl className="schedule-row">
          <div>
            <dt>{selected.flight.origin_airport_code ?? "Origin"}</dt>
            <dd>{formatZulu(selected.flight.scheduled_departure_at)}</dd>
          </div>
          <span aria-hidden="true">→</span>
          <div>
            <dt>{selected.flight.destination_airport_code ?? "Destination"}</dt>
            <dd>{formatZulu(selected.flight.scheduled_arrival_at)}</dd>
          </div>
        </dl>
      </section>

      <section className="detail-section timeline-section" aria-labelledby="timeline-title">
        <div className="detail-section-title">
          <h3 id="timeline-title">Operational timeline</h3>
          <span>{timeline.length} events</span>
        </div>
        {timelineState === "loading" ? (
          <TimelineSkeleton />
        ) : timelineState === "error" ? (
          <p className="timeline-message" role="status">
            Timeline unavailable. Current flight state remains visible.
          </p>
        ) : timeline.length === 0 ? (
          <p className="timeline-message">No source-attributed events for this flight yet.</p>
        ) : (
          <ol className="timeline-list">
            {timeline.slice().reverse().map((event) => (
              <li key={event.id}>
                <span className="timeline-node" aria-hidden="true" />
                <div>
                  <strong>{eventLabel(event)}</strong>
                  <p>
                    {formatZulu(event.event_time)} · {event.source?.provider ?? "Internal"}
                  </p>
                </div>
              </li>
            ))}
          </ol>
        )}
      </section>

      <div className="source-strip">
        <span>Source</span>
        <strong>{position?.source.provider ?? selected.flight.source.provider}</strong>
        <code>{(position?.source.envelope_id ?? selected.flight.source.envelope_id).slice(0, 8)}</code>
      </div>
    </aside>
  );
}

function eventLabel(event: FleetEvent): string {
  const labels: Record<string, string> = {
    aircraft_position: "Position update",
    flight: "Flight state update",
    planned_route: "Route revision",
    alert: "Operational alert",
  };
  return labels[event.event.event_type] ?? event.event.event_type.replaceAll("_", " ");
}

function TimelineSkeleton() {
  return (
    <div className="timeline-skeleton" aria-label="Loading timeline" role="status">
      <span />
      <span />
      <span />
    </div>
  );
}
