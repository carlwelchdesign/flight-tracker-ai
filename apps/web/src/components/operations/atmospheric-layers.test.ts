import { describe, expect, it } from "vitest";
import { ATMOSPHERIC_RASTER_LAYERS } from "./atmospheric-layers";

describe("NOAA atmospheric raster configuration", () => {
  it("uses only fixed transparent HTTPS nowCOAST WMS layers", () => {
    const layers = Object.values(ATMOSPHERIC_RASTER_LAYERS);
    expect(layers).toHaveLength(3);
    for (const layer of layers) {
      const url = new URL(layer.tileUrl);
      expect(url.origin).toBe("https://nowcoast.noaa.gov");
      expect(url.pathname).toMatch(/^\/geoserver\/(satellite|weather_radar|ndfd_wind)\/wms$/);
      expect(url.searchParams.get("request")).toBe("GetMap");
      expect(url.searchParams.get("transparent")).toBe("true");
      expect(url.searchParams.get("bbox")).toBe("{bbox-epsg-3857}");
      expect(layer.tileUrl).toContain("bbox={bbox-epsg-3857}");
    }
    expect(ATMOSPHERIC_RASTER_LAYERS.radar.tileUrl).toContain("base_reflectivity_mosaic");
    expect(ATMOSPHERIC_RASTER_LAYERS.satellite.tileUrl).toContain("goes_longwave_imagery");
    expect(ATMOSPHERIC_RASTER_LAYERS.surfaceWind.tileUrl).toContain("wind_velocity");
  });
});
