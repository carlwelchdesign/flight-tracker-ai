import { describe, expect, it } from "vitest";
import { DEFAULT_PUBLIC_LIVE_REGION, PUBLIC_LIVE_REGIONS, findPublicLiveRegion } from "./public-live-regions";

describe("public live regions", () => {
  it("offers the curated airport catalog with SFO as the default", () => {
    expect(DEFAULT_PUBLIC_LIVE_REGION.code).toBe("sfo");
    expect(PUBLIC_LIVE_REGIONS.map((region) => region.airport)).toEqual([
      "SFO", "LAX", "SEA", "DEN", "ORD", "ATL", "JFK",
    ]);
  });

  it("rejects identifiers outside the curated catalog", () => {
    expect(findPublicLiveRegion("LAX")?.name).toBe("Los Angeles");
    expect(findPublicLiveRegion("moon")).toBeNull();
  });
});
