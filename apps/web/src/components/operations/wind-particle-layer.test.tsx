import { afterEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import type { Map as MapLibreMap } from "maplibre-gl";
import type { PublicWindField } from "@/lib/public-atmosphere";
import { WindParticleLayer } from "./wind-particle-layer";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("WindParticleLayer motion preference", () => {
  it("draws a static vector field and schedules no animation when motion is reduced", () => {
    const context = canvasContext();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(context);
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: true })));
    const frame = vi.spyOn(window, "requestAnimationFrame");

    render(<WindParticleLayer map={mapStub()} field={field()} visible />);

    expect(context.stroke).toHaveBeenCalledTimes(1);
    expect(frame).not.toHaveBeenCalled();
  });
});

function mapStub() {
  return {
    getContainer: () => ({ clientWidth: 800, clientHeight: 500 }),
    project: () => ({ x: 200, y: 150 }),
    on: vi.fn(),
    off: vi.fn(),
  } as unknown as MapLibreMap;
}

function canvasContext() {
  return {
    canvas: { width: 800, height: 500 },
    setTransform: vi.fn(),
    clearRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
  } as unknown as CanvasRenderingContext2D;
}

function field(): PublicWindField {
  return {
    state: "current",
    retained: false,
    region_code: "sfo",
    region_name: "San Francisco",
    level: { code: "500", label: "500 hPa", pressure_hpa: 500, approximate_altitude_feet: 18_400 },
    generated_at: "2026-07-22T00:55:00Z",
    forecast_time: "2026-07-22T00:45:00Z",
    last_success_at: "2026-07-22T00:55:00Z",
    last_error_code: null,
    attribution: {
      provider: "Open-Meteo",
      model: "NOAA GFS / HRRR",
      source_url: "https://open-meteo.com/",
      license_url: "https://open-meteo.com/en/license",
      text: "NOAA GFS/HRRR model data delivered by Open-Meteo",
    },
    samples: [{
      latitude_degrees: 37.6,
      longitude_degrees: -122.4,
      speed_knots: 42,
      direction_from_degrees: 270,
    }],
  };
}
