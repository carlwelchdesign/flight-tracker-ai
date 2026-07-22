import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { PublicWeatherSnapshot } from "@/lib/public-weather";
import { PublicWeatherOverlay } from "./public-weather-overlay";

describe("public weather overlay", () => {
  it("labels retained evidence as the last NOAA picture and offers retry", async () => {
    const retry = vi.fn();
    render(
      <PublicWeatherOverlay
        snapshot={snapshot()}
        state="current"
        retained
        showHazards
        showObservations
        selection={null}
        onShowHazards={vi.fn()}
        onShowObservations={vi.fn()}
        onSelect={vi.fn()}
        onRetry={retry}
      />,
    );

    expect(screen.getByText("Last NOAA picture")).toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("shows an explicit unavailable empty state without fabricating evidence", () => {
    render(
      <PublicWeatherOverlay
        snapshot={null}
        state="unavailable"
        retained={false}
        showHazards
        showObservations
        selection={null}
        onShowHazards={vi.fn()}
        onShowObservations={vi.fn()}
        onSelect={vi.fn()}
        onRetry={vi.fn()}
      />,
    );

    expect(screen.getByText("NOAA unavailable")).toBeInTheDocument();
    expect(screen.getByText("No accepted weather evidence")).toBeInTheDocument();
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("exposes atmospheric toggles and an allowlisted model level", async () => {
    const setLevel = vi.fn();
    const setSatellite = vi.fn();
    render(
      <PublicWeatherOverlay
        snapshot={snapshot()}
        state="current"
        retained={false}
        showHazards
        showObservations
        selection={null}
        onShowHazards={vi.fn()}
        onShowObservations={vi.fn()}
        onSelect={vi.fn()}
        onRetry={vi.fn()}
        atmosphere={{
          showRadar: true,
          showSatellite: true,
          showSurfaceWind: false,
          showModelWind: true,
          windLevel: "500",
          windState: "loading",
          windField: null,
          onShowRadar: vi.fn(),
          onShowSatellite: setSatellite,
          onShowSurfaceWind: vi.fn(),
          onShowModelWind: vi.fn(),
          onWindLevel: setLevel,
        }}
      />,
    );

    expect(screen.getByRole("group", { name: "Atmospheric layers" })).toBeInTheDocument();
    await userEvent.setup().click(screen.getByRole("checkbox", { name: "Satellite clouds" }));
    expect(setSatellite).toHaveBeenCalledWith(false);
    await userEvent.setup().selectOptions(screen.getByRole("combobox", { name: "Model wind level" }), "300");
    expect(setLevel).toHaveBeenCalledWith("300");
    expect(screen.getByText("Loading model wind")).toBeInTheDocument();
  });
});

function snapshot(): PublicWeatherSnapshot {
  return {
    state: "current",
    generated_at: "2026-07-21T23:00:00Z",
    attribution: { text: "NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
    sources: [],
    hazards: [],
    observations: [],
  };
}
