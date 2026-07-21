export type BackendHealth = {
  status: "ok" | "degraded";
  service: string;
  version: string;
  checks: {
    critical_workers: "ok" | "degraded";
  };
  workers: WorkerHealth[];
};

export type WorkerHealth = {
  name: string;
  state: "starting" | "running" | "stale" | "failed" | "stopped";
  last_heartbeat_at: string | null;
  detail: string | null;
};

export function parseBackendHealth(value: unknown): BackendHealth {
  if (!isBackendHealth(value)) {
    throw new Error("API returned an unexpected health payload");
  }
  return value;
}

function isBackendHealth(value: unknown): value is BackendHealth {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    ["ok", "degraded"].includes(String(candidate.status)) &&
    typeof candidate.service === "string" &&
    typeof candidate.version === "string" &&
    isRecord(candidate.checks) &&
    ["ok", "degraded"].includes(String(candidate.checks.critical_workers)) &&
    Array.isArray(candidate.workers) &&
    candidate.workers.every(isWorkerHealth)
  );
}

function isWorkerHealth(value: unknown): value is WorkerHealth {
  return (
    isRecord(value) &&
    typeof value.name === "string" &&
    ["starting", "running", "stale", "failed", "stopped"].includes(String(value.state)) &&
    (value.last_heartbeat_at === null || typeof value.last_heartbeat_at === "string") &&
    (value.detail === null || typeof value.detail === "string")
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
