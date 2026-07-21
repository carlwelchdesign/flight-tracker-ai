import type { ConnectionState, ServiceHealthState } from "./operations-health-model";

type OperationsBadgesProps = {
  connection: ConnectionState;
  serviceHealth: ServiceHealthState;
};

export function OperationsBadges({ connection, serviceHealth }: OperationsBadgesProps) {
  const connectionLabels: Record<ConnectionState, string> = {
    connecting: "Stream connecting",
    live: "Stream live",
    reconnecting: "Stream reconnecting",
    disconnected: "Stream disconnected",
  };
  const systemLabel =
    serviceHealth.state === "checking"
      ? "Service checking"
      : serviceHealth.state === "healthy"
        ? "Service healthy"
        : "Service degraded";

  return (
    <div className="operations-badges" aria-label="Service and source status">
      <span className={`system-badge system-${serviceHealth.state}`} role="status">
        <i aria-hidden="true" /> {systemLabel}
      </span>
      <span className={`connection-badge connection-${connection}`} role="status">
        <i aria-hidden="true" /> {connectionLabels[connection]}
      </span>
    </div>
  );
}
