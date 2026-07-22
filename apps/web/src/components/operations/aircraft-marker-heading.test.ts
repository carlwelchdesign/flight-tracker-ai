import { describe, expect, it } from "vitest";
import { liveMarkerRotationDegrees } from "./aircraft-marker-heading";

describe("live aircraft marker heading", () => {
  it.each([
    [0, -90],
    [90, 0],
    [180, 90],
    [270, 180],
  ])("offsets a %i-degree true heading to %i degrees for the marker glyph", (heading, rotation) => {
    expect(liveMarkerRotationDegrees(heading)).toBe(rotation);
  });

  it("keeps a deterministic glyph orientation when heading is unavailable", () => {
    expect(liveMarkerRotationDegrees(null)).toBe(-90);
  });
});
