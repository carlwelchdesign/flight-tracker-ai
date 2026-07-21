import { describe, expect, it } from "vitest";
import { parseBackendHealth } from "./backend-health";

describe("backend health contract", () => {
  it("accepts structured critical-worker health", () => {
    expect(
      parseBackendHealth({
        status: "degraded",
        service: "flight-tracker-api",
        version: "0.1.0",
        checks: { critical_workers: "degraded" },
        workers: [
          {
            name: "fleet_projection",
            state: "stale",
            last_heartbeat_at: "2026-07-20T16:04:30Z",
            detail: null,
          },
        ],
      }),
    ).toMatchObject({ status: "degraded", workers: [{ state: "stale" }] });
  });

  it("rejects unknown worker states", () => {
    expect(() =>
      parseBackendHealth({
        status: "ok",
        service: "flight-tracker-api",
        version: "0.1.0",
        checks: { critical_workers: "ok" },
        workers: [
          {
            name: "fleet_projection",
            state: "probably_fine",
            last_heartbeat_at: null,
            detail: null,
          },
        ],
      }),
    ).toThrow("API returned an unexpected health payload");
  });
});
