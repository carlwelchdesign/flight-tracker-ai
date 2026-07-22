import { describe, expect, it } from "vitest";
import { nearestWindSample, windVector } from "./wind-particle-field";

describe("wind particle field policy", () => {
  it("converts meteorological direction-from into motion-toward vectors", () => {
    expect(windVector({ latitude_degrees: 0, longitude_degrees: 0, speed_knots: 20, direction_from_degrees: 270 }))
      .toEqual(expect.objectContaining({ east: expect.closeTo(20, 5), north: expect.closeTo(0, 5) }));
    expect(windVector({ latitude_degrees: 0, longitude_degrees: 0, speed_knots: 10, direction_from_degrees: 0 }))
      .toEqual(expect.objectContaining({ east: expect.closeTo(0, 5), north: expect.closeTo(-10, 5) }));
  });

  it("uses the closest bounded source sample", () => {
    const west = { latitude_degrees: 37, longitude_degrees: -123, speed_knots: 10, direction_from_degrees: 90 };
    const east = { latitude_degrees: 37, longitude_degrees: -121, speed_knots: 30, direction_from_degrees: 270 };
    expect(nearestWindSample([west, east], -121.1, 37)).toBe(east);
  });
});
