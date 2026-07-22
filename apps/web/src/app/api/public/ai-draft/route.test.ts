import { beforeEach, describe, expect, it, vi } from "vitest";
import { GET } from "./route";

describe("public AI draft proxy", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    process.env.API_BASE_URL = "https://api.example.test";
  });

  it("proxies the fixed backend route without caching", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response("{}", { status: 200 }));
    const response = await GET();
    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.test/api/public/ai-draft",
      expect.objectContaining({ cache: "no-store" }),
    );
    expect(response.headers.get("cache-control")).toBe("no-store");
  });
});
