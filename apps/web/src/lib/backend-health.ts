export type BackendHealth = {
  status: "ok";
  service: string;
  version: string;
};

export type BackendStatus =
  | { state: "connected"; health: BackendHealth }
  | { state: "degraded"; message: string };

const DEFAULT_API_BASE_URL = "http://localhost:8080";

export async function getBackendStatus(): Promise<BackendStatus> {
  const apiBaseUrl = process.env.API_BASE_URL ?? DEFAULT_API_BASE_URL;

  try {
    const response = await fetch(`${apiBaseUrl}/health`, {
      cache: "no-store",
      signal: AbortSignal.timeout(2_000),
    });

    if (!response.ok) {
      return {
        state: "degraded",
        message: `API returned HTTP ${response.status}`,
      };
    }

    const payload: unknown = await response.json();
    if (!isBackendHealth(payload)) {
      return {
        state: "degraded",
        message: "API returned an unexpected health payload",
      };
    }

    return { state: "connected", health: payload };
  } catch {
    return {
      state: "degraded",
      message: "API is unavailable",
    };
  }
}

function isBackendHealth(value: unknown): value is BackendHealth {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    candidate.status === "ok" &&
    typeof candidate.service === "string" &&
    typeof candidate.version === "string"
  );
}
