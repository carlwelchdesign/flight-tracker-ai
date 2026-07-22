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
