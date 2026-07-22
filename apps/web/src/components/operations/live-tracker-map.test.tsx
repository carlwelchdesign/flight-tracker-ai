import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_PUBLIC_LIVE_REGION } from "@/lib/public-live-regions";
import { DEFAULT_PUBLIC_WEATHER_LAYERS } from "@/lib/public-tracker-url";
import type { PublicAircraft } from "@/lib/public-live-positions";
import { LiveTrackerMap } from "./live-tracker-map";

const maplibre = vi.hoisted(() => {
  class MapMock {
    static instances: MapMock[] = [];
    options: Record<string, unknown>;
    handlers = new Map<string, (...args: unknown[]) => void>();
    sources = new Map<string, { setData: ReturnType<typeof vi.fn> }>();
    addControl = vi.fn();
    addLayer = vi.fn();
    setLayoutProperty = vi.fn();
    easeTo = vi.fn();
    fitBounds = vi.fn();
    resize = vi.fn();
    remove = vi.fn();
    center = { lng: -122.379, lat: 37.6213 };

    constructor(options: Record<string, unknown>) {
      this.options = options;
      MapMock.instances.push(this);
    }

    on(event: string, layerOrHandler: string | ((...args: unknown[]) => void), handler?: (...args: unknown[]) => void) {
      if (typeof layerOrHandler === "function") this.handlers.set(event, layerOrHandler);
      else if (handler) this.handlers.set(`${event}:${layerOrHandler}`, handler);
      return this;
    }

    trigger(event: string, value?: unknown) {
      this.handlers.get(event)?.(value);
    }

    addSource(id: string) {
      this.sources.set(id, { setData: vi.fn() });
    }

    getSource(id: string) {
      return this.sources.get(id);
    }

    getLayer() {
      return {};
    }

    getCenter() {
      return this.center;
    }

    getZoom() {
      return 7.5;
    }

    getBearing() {
      return 0;
    }

    getPitch() {
      return 18;
    }

    getCanvas() {
      return document.createElement("canvas");
    }
  }

  class MarkerMock {
    static instances: MarkerMock[] = [];
    element: HTMLButtonElement;
    location = { lng: 0, lat: 0 };
    setLngLat = vi.fn((value: [number, number]) => {
      this.location = { lng: value[0], lat: value[1] };
      return this;
    });
    addTo = vi.fn(() => this);
    remove = vi.fn();

    constructor(options: { element: HTMLButtonElement }) {
      this.element = options.element;
      MarkerMock.instances.push(this);
    }

    getLngLat() {
      return this.location;
    }
  }

  return {
    Map: MapMock,
    Marker: MarkerMock,
    NavigationControl: class NavigationControlMock {},
    FullscreenControl: class FullscreenControlMock {},
  };
});

vi.mock("maplibre-gl", () => maplibre);

class ResizeObserverMock {
  static instances: ResizeObserverMock[] = [];
  observe = vi.fn();
  disconnect = vi.fn();

  constructor(callback: ResizeObserverCallback) {
    void callback;
    ResizeObserverMock.instances.push(this);
  }
}

describe("LiveTrackerMap lifecycle and motion", () => {
  let reducedMotion = false;
  let frames: FrameRequestCallback[];

  beforeEach(() => {
    maplibre.Map.instances.length = 0;
    maplibre.Marker.instances.length = 0;
    ResizeObserverMock.instances.length = 0;
    frames = [];
    reducedMotion = false;
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);
    vi.stubGlobal("matchMedia", vi.fn(() => ({ matches: reducedMotion })));
    vi.stubGlobal("requestAnimationFrame", vi.fn((callback: FrameRequestCallback) => {
      frames.push(callback);
      return frames.length;
    }));
    vi.stubGlobal("cancelAnimationFrame", vi.fn());
    vi.spyOn(performance, "now").mockReturnValue(100);
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("initializes the navigable map, exposes marker selection, and cleans up resources", async () => {
    const onSelect = vi.fn();
    const { unmount } = renderMap([aircraft()], onSelect);

    await waitFor(() => expect(maplibre.Map.instances).toHaveLength(1));
    const map = maplibre.Map.instances[0];
    expect(map.options.style).toBe("https://tiles.openfreemap.org/styles/dark");
    expect(map.addControl).toHaveBeenCalledTimes(2);

    act(() => map.trigger("load"));
    await waitFor(() => expect(screen.queryByText("Loading navigable map…")).not.toBeInTheDocument());
    await waitFor(() => expect(maplibre.Marker.instances).toHaveLength(1));

    const marker = maplibre.Marker.instances[0];
    expect(marker.element).toHaveAttribute("aria-label", expect.stringContaining("Select UAL123"));
    expect(marker.element.style.getPropertyValue("--aircraft-heading")).toBe("180deg");
    marker.element.click();
    expect(onSelect).toHaveBeenCalledWith("aircraft-1");

    unmount();
    expect(marker.remove).toHaveBeenCalledOnce();
    expect(map.remove).toHaveBeenCalledOnce();
    expect(ResizeObserverMock.instances[0].disconnect).toHaveBeenCalledOnce();
  });

  it("interpolates accepted positions but updates immediately for reduced motion", async () => {
    const { rerender } = renderMap([aircraft()]);
    await waitFor(() => expect(maplibre.Map.instances).toHaveLength(1));
    act(() => maplibre.Map.instances[0].trigger("load"));
    await waitFor(() => expect(maplibre.Marker.instances).toHaveLength(1));
    const marker = maplibre.Marker.instances[0];

    rerender(component([aircraft(-122.0, 37.9)]));
    await waitFor(() => expect(frames).toHaveLength(1));
    expect(marker.location).toEqual({ lng: -122.2, lat: 37.6 });
    act(() => frames.shift()?.(1_500));
    expect(marker.location.lng).toBeCloseTo(-122.0);
    expect(marker.location.lat).toBeCloseTo(37.9);

    reducedMotion = true;
    rerender(component([aircraft(-121.8, 38.0)]));
    await waitFor(() => expect(marker.location).toEqual({ lng: -121.8, lat: 38.0 }));
    expect(frames).toHaveLength(0);
  });

  it("surfaces map failures without removing the accessible map boundary", async () => {
    renderMap([]);
    await waitFor(() => expect(maplibre.Map.instances).toHaveLength(1));
    act(() => maplibre.Map.instances[0].trigger("error", { error: new Error("tiles") }));

    expect(await screen.findByText("Basemap temporarily unavailable")).toHaveAttribute("role", "status");
    expect(screen.getByLabelText(/Interactive aircraft map/i)).toBeInTheDocument();
  });
});

function renderMap(items: PublicAircraft[], onSelect = vi.fn()) {
  return render(component(items, onSelect));
}

function component(items: PublicAircraft[], onSelect = vi.fn()) {
  return (
    <LiveTrackerMap
      aircraft={items}
      region={DEFAULT_PUBLIC_LIVE_REGION}
      selectedId={items[0]?.id ?? null}
      status={null}
      mode="live"
      trail={[]}
      projection={null}
      weather={null}
      weatherState="unavailable"
      weatherRetained={false}
      view={null}
      layers={{
        ...DEFAULT_PUBLIC_WEATHER_LAYERS,
        radar: false,
        satellite: false,
        surfaceWind: false,
        modelWind: false,
      }}
      onRetryWeather={vi.fn()}
      onSelect={onSelect}
      onViewChange={vi.fn()}
      onLayersChange={vi.fn()}
    />
  );
}

function aircraft(longitude = -122.2, latitude = 37.6): PublicAircraft {
  return {
    id: "aircraft-1",
    callsign: "UAL123",
    aircraft_registration: null,
    icao_hex: "ABC123",
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    altitude: { value: 12_000, unit: "feet", reference: "barometric" },
    heading_true_degrees: 270,
    ground_speed: { value: 310, unit: "knots" },
    quality: "observed",
    observed_at: "2026-07-22T15:00:00Z",
    received_at: "2026-07-22T15:00:02Z",
    provider: "adsb.lol",
  };
}
