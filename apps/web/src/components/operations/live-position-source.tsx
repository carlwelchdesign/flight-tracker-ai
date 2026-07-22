import type { LivePositionStatus } from "@/lib/live-positions-api";

import { formatZulu } from "./operations-model";

type LivePositionSourceProps = {
  status: LivePositionStatus | null;
  message: string | null;
  liveFlightsVisible: boolean;
  replayAvailable: boolean;
  onUseReplay: () => void;
  onRetry: () => void;
};

export function LivePositionSource({
  status,
  message,
  liveFlightsVisible,
  replayAvailable,
  onUseReplay,
  onRetry,
}: LivePositionSourceProps) {
  const state = status?.state ?? "unavailable";
  const showAttribution = Boolean(status?.attribution && (status.enabled || liveFlightsVisible));
  const role = state === "unavailable" ? "alert" : "status";

  return (
    <section
      className={`live-position-source live-position-${state}`}
      aria-labelledby="live-position-title"
      role={role}
    >
      <div className="live-position-summary">
        <span className="live-position-indicator" aria-hidden="true" />
        <div>
          <p className="ops-eyebrow">Optional live position layer</p>
          <h2 id="live-position-title">{sourceTitle(status)}</h2>
          <p>{sourceDescription(status, message)}</p>
        </div>
      </div>

      {status?.enabled && (
        <dl className="live-position-metrics">
          <div><dt>Aircraft</dt><dd>{status.aircraft_count}</dd></div>
          <div><dt>Current</dt><dd>{status.fresh_position_count}</dd></div>
          <div><dt>Stale</dt><dd>{status.stale_position_count}</dd></div>
          <div><dt>Rejected</dt><dd>{status.rejected_record_count}</dd></div>
          <div><dt>As of</dt><dd>{formatZulu(status.observed_at)}</dd></div>
          <div>
            <dt>Region</dt>
            <dd>{status.region ? `${status.region.radius_nautical_miles} NM radius` : "—"}</dd>
          </div>
        </dl>
      )}

      <div className="live-position-actions">
        {(state === "degraded" || state === "unavailable") && replayAvailable && (
          <button type="button" onClick={onUseReplay}>Use replay view</button>
        )}
        {state === "unavailable" && <button type="button" onClick={onRetry}>Retry status</button>}
      </div>

      {showAttribution && status?.attribution && (
        <p className="live-position-attribution">
          <a href={status.attribution.source_url} target="_blank" rel="noreferrer">
            {status.attribution.source_name}
          </a>{" "}
          live positions ·{" "}
          <a href={status.attribution.terms_url} target="_blank" rel="noreferrer">
            {status.attribution.terms_label}
          </a>
        </p>
      )}
    </section>
  );
}

function sourceTitle(status: LivePositionStatus | null): string {
  if (!status) return "Status unavailable · replay preserved";
  const titles: Record<LivePositionStatus["state"], string> = {
    disabled: "Off · deterministic replay is the default",
    connecting: "Connecting to best-effort positions",
    current: `${providerName(status.provider)} positions available`,
    degraded: "Live positions degraded · replay preserved",
    unavailable: "Live positions unavailable · replay preserved",
  };
  return titles[status.state];
}

function providerName(provider: string | null): string {
  if (provider === "adsb.lol") return "ADSB.lol";
  if (provider === "airplanes.live") return "Airplanes.live fallback";
  return "Live";
}

function sourceDescription(status: LivePositionStatus | null, message: string | null): string {
  if (!status) return message ?? "The optional source could not be checked.";
  if (status.state === "disabled") {
    return "No external aircraft-position request is made. The complete demonstration remains available through versioned simulation.";
  }
  if (status.state === "connecting") {
    return "The regional Rust adapter is waiting for its first bounded, uncached response.";
  }
  if (status.state === "current") {
    return `${status.fresh_position_count} positions meet the ${status.stale_after_seconds}-second freshness threshold. Coverage is crowdsourced and incomplete.`;
  }
  const error = status.last_error_code ? ` Source condition: ${status.last_error_code.replaceAll("_", " ")}.` : "";
  return `Replay remains usable. Any previously accepted position picture stays visible, but live completeness is not claimed.${error}`;
}
