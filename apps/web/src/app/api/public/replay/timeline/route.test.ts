import { afterEach, describe, expect, it, vi } from "vitest";
import { GET } from "./route";

vi.mock("@/lib/fleet-api", () => ({ getApiBaseUrl: () => "https://api.example.test" }));

describe("public replay timeline proxy", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("forwards the public timeline without credentials", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ schema_version: 1 }), {
      status: 200,
      headers: { "content-type": "application/json" },
    }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET();

    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe("no-store");
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/replay/timeline",
      expect.objectContaining({ cache: "no-store" }),
    );
    expect(fetchMock.mock.calls[0][1]).not.toHaveProperty("headers");
  });

  it("fails closed when the Rust API cannot be reached", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
    const response = await GET();
    expect(response.status).toBe(503);
    expect(await response.json()).toEqual({
      error: {
        code: "replay_timeline_unavailable",
        message: "The deterministic replay timeline is unavailable",
      },
    });
  });
});
