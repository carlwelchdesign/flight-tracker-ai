import type { ConnectionState, ServiceHealthState } from "./operations-health-model";

type OperationsStatusRegionProps = {
  connection: ConnectionState;
  serviceHealth: ServiceHealthState;
  feedOutage: boolean;
  error: string | null;
  onRetry: () => void;
  onDismiss: () => void;
  onRestoreFeed: () => void;
};

export function OperationsStatusRegion({
  connection,
  serviceHealth,
  feedOutage,
  error,
  onRetry,
  onDismiss,
  onRestoreFeed,
}: OperationsStatusRegionProps) {
  return (
    <div className="operations-status-region" aria-live="polite">
      {feedOutage && (
        <StatusBanner tone="error" title="Simulation feed outage">
          Source updates are intentionally suspended; the last accepted picture remains visible.{" "}
          <button type="button" onClick={onRestoreFeed}>Restore feed</button>
        </StatusBanner>
      )}
      {serviceHealth.state === "degraded" && (
        <StatusBanner tone="error" title="Critical service degraded">
          {serviceHealth.message}
        </StatusBanner>
      )}
      {connection === "reconnecting" && !feedOutage && (
        <StatusBanner tone="stale" title="Live stream interrupted">
          Showing the last accepted operational picture while reconnection is attempted.
        </StatusBanner>
      )}
      {connection === "disconnected" && (
        <StatusBanner tone="error" title="Operations API disconnected">
          {error ?? "No live source is available."}{" "}
          <button type="button" onClick={onRetry}>Retry</button>
        </StatusBanner>
      )}
      {error && connection !== "disconnected" && (
        <StatusBanner tone="error" title="Partial data issue">
          {error} <button type="button" onClick={onDismiss}>Dismiss</button>
        </StatusBanner>
      )}
    </div>
  );
}

function StatusBanner({
  tone,
  title,
  children,
}: {
  tone: "stale" | "error";
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className={`status-banner status-${tone}`} role={tone === "error" ? "alert" : "status"}>
      <strong>{title}</strong>
      <span>{children}</span>
    </div>
  );
}
