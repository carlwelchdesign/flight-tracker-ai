import { afterEach, describe, expect, it, vi } from "vitest";
import { GET } from "./route";

afterEach(() => vi.unstubAllGlobals());

describe("public airport intelligence proxy", () => {
  it("forwards only an allowlisted airport to Rust", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200, headers: { "cache-control": "public, max-age=30" } }));
    vi.stubGlobal("fetch", fetchMock);
    const response = await GET(new Request("https://web.example.test/api/public/airport-intelligence?airport=kden&bbox=world"));
    expect(response.status).toBe(200);
    expect(fetchMock).toHaveBeenCalledWith(expect.stringMatching(/\/api\/public\/airport-intelligence\?airport=KDEN$/), expect.anything());
    expect(fetchMock.mock.calls[0][0]).not.toContain("bbox");
  });

  it("rejects unknown airports without contacting Rust", async () => {
    const fetchMock = vi.fn(); vi.stubGlobal("fetch", fetchMock);
    const response = await GET(new Request("https://web.example.test/api/public/airport-intelligence?airport=EGLL"));
    expect(response.status).toBe(400); expect(fetchMock).not.toHaveBeenCalled();
  });
});
