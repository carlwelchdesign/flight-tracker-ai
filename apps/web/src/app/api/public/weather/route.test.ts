import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/fleet-api", () => ({ getApiBaseUrl: () => "https://api.example.test" }));

import { GET } from "./route";

describe("public weather proxy", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("forwards the public endpoint without credentials and preserves no-store", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      state: "current", generated_at: "2026-07-21T23:00:00Z", sources: [], hazards: [], observations: [],
    }), { headers: { "content-type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET();

    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/weather",
      expect.objectContaining({ cache: "no-store" }),
    );
    const options = fetchMock.mock.calls[0][1] as RequestInit;
    expect(options.headers).toBeUndefined();
  });

  it("fails closed without caching when the API cannot be reached", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    const response = await GET();

    expect(response.status).toBe(503);
    expect(response.headers.get("cache-control")).toBe("no-store");
  });
});
