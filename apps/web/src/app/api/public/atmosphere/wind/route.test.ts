import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/fleet-api", () => ({ getApiBaseUrl: () => "https://api.example.test" }));

import { GET } from "./route";

describe("public atmospheric wind proxy", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("forwards an allowlisted region and pressure level without caching", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ state: "current" }), { status: 200, headers: { "content-type": "application/json" } }),
    );
    const response = await GET(new Request("https://web.example.test/api/public/atmosphere/wind?region=lax&level=500"));
    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/atmosphere/wind?region=lax&level=500",
      expect.objectContaining({ cache: "no-store" }),
    );
  });

  it("rejects arbitrary coordinates, regions, and levels before the backend", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch");
    const response = await GET(new Request("https://web.example.test/api/public/atmosphere/wind?region=world&level=450&lat=1"));
    expect(response.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
