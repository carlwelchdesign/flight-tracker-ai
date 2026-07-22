import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/fleet-api", () => ({ getApiBaseUrl: () => "https://api.example.test" }));

import { GET } from "./route";

describe("public replay attention proxy", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("forwards the public endpoint without credentials", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      schema_version: 1,
      scenario_id: "m1-operations-v1",
      scenario_time: "2026-07-21T16:01:00Z",
      source: "portfolio deterministic replay",
      aircraft: [],
    }), { headers: { "content-type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET();

    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/replay/attention",
      expect.objectContaining({ cache: "no-store" }),
    );
    const options = fetchMock.mock.calls[0][1] as RequestInit;
    expect(options.headers).toBeUndefined();
  });

  it("fails closed when the Rust API cannot be reached", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    const response = await GET();

    expect(response.status).toBe(503);
    expect(response.headers.get("cache-control")).toBe("no-store");
  });
});
