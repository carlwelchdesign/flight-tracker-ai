import type { Map as MapLibreMap } from "maplibre-gl";

export type AtmosphericRasterLayer = "radar" | "satellite" | "surfaceWind";

type RasterDefinition = {
  sourceId: string;
  layerId: string;
  label: string;
  tileUrl: string;
  opacity: number;
};

const NOWCOAST = "https://nowcoast.noaa.gov/geoserver";

export const ATMOSPHERIC_RASTER_LAYERS: Record<AtmosphericRasterLayer, RasterDefinition> = {
  satellite: {
    sourceId: "noaa-nowcoast-satellite-source",
    layerId: "noaa-nowcoast-satellite",
    label: "GOES longwave cloud imagery",
    tileUrl: wmsTileUrl("satellite", "goes_longwave_imagery"),
    opacity: 0.42,
  },
  radar: {
    sourceId: "noaa-nowcoast-radar-source",
    layerId: "noaa-nowcoast-radar",
    label: "MRMS base reflectivity",
    tileUrl: wmsTileUrl("weather_radar", "base_reflectivity_mosaic"),
    opacity: 0.66,
  },
  surfaceWind: {
    sourceId: "noaa-nowcoast-surface-wind-source",
    layerId: "noaa-nowcoast-surface-wind",
    label: "NDFD surface wind barbs",
    tileUrl: wmsTileUrl("ndfd_wind", "wind_velocity", "wind_arrows_from_uv"),
    opacity: 0.82,
  },
};

export function addAtmosphericRasterLayers(map: MapLibreMap) {
  for (const definition of Object.values(ATMOSPHERIC_RASTER_LAYERS)) {
    map.addSource(definition.sourceId, {
      type: "raster",
      tiles: [definition.tileUrl],
      tileSize: 256,
      attribution: "NOAA nowCOAST · Not for navigation",
    });
    map.addLayer({
      id: definition.layerId,
      type: "raster",
      source: definition.sourceId,
      layout: { visibility: "none" },
      paint: { "raster-opacity": definition.opacity, "raster-fade-duration": 280 },
    });
  }
}

export function setAtmosphericRasterVisibility(
  map: MapLibreMap,
  layer: AtmosphericRasterLayer,
  visible: boolean,
) {
  const definition = ATMOSPHERIC_RASTER_LAYERS[layer];
  if (map.getLayer(definition.layerId)) {
    map.setLayoutProperty(definition.layerId, "visibility", visible ? "visible" : "none");
  }
}

function wmsTileUrl(service: string, layer: string, style = "") {
  const params = new URLSearchParams({
    service: "WMS",
    version: "1.1.1",
    request: "GetMap",
    layers: layer,
    styles: style,
    format: "image/png",
    transparent: "true",
    srs: "EPSG:3857",
    bbox: "{bbox-epsg-3857}",
    width: "256",
    height: "256",
  });
  const query = params.toString().replace(
    "%7Bbbox-epsg-3857%7D",
    "{bbox-epsg-3857}",
  );
  return `${NOWCOAST}/${service}/wms?${query}`;
}
