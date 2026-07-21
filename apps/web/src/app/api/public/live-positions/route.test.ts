import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/fleet-api", () => ({ getApiBaseUrl: () => "https://api.example.test" }));

import { GET } from "./route";

describe("public live-position proxy", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("forwards the public endpoint without credentials and preserves no-store", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ status: {}, data: [] }), {
      headers: { "cache-control": "no-store", "content-type": "application/json" },
    }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET();

    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/live-positions",
      expect.objectContaining({ cache: "no-store" }),
    );
    const options = fetchMock.mock.calls[0][1] as RequestInit;
    expect(options.headers).toBeUndefined();
  });
});
