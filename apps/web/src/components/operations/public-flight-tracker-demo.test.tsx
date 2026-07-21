import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PublicFlightTrackerDemo } from "./public-flight-tracker-demo";

describe("public flight tracker demo", () => {
  it("shows the navigable map, replay fallback, aircraft detail, and protected console link", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 503 })));
    const user = userEvent.setup();
    render(<PublicFlightTrackerDemo />);

    expect(screen.getByRole("heading", { name: "Bay Area traffic" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Aircraft" })).toBeInTheDocument();
    expect(await screen.findByText(/replay demonstration/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "FT101" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /review an alert/i })).not.toBeInTheDocument();
    expect(screen.getByText(/sign in to review alerts and protected actions/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /protected operations console/i })).toHaveAttribute(
      "href",
      "/sign-in",
    );

    await user.click(screen.getByRole("button", { name: /FT303/i }));
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("labels live source evidence and visual interpolation honestly", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({
      status: {
        enabled: true,
        provider: "adsb.lol",
        state: "current",
        best_effort: true,
        observed_at: "2026-07-21T22:10:56Z",
        last_success_at: "2026-07-21T22:10:56Z",
        newest_position_at: "2026-07-21T22:10:50Z",
        aircraft_count: 1,
        fresh_position_count: 1,
        stale_position_count: 0,
        stale_after_seconds: 300,
        region: { latitude_degrees: 37.62, longitude_degrees: -122.38, radius_nautical_miles: 50 },
        attribution: null,
      },
      data: [{
        id: "aircraft-1",
        callsign: "UAL123",
        aircraft_registration: null,
        longitude_degrees: -122.2,
        latitude_degrees: 37.6,
        altitude: { value: 12000, unit: "feet", reference: "ellipsoid" },
        heading_true_degrees: 270,
        ground_speed: { value: 310, unit: "knots" },
        quality: "observed",
        observed_at: new Date().toISOString(),
        received_at: new Date().toISOString(),
        provider: "adsb.lol",
      }],
    }), { status: 200, headers: { "Content-Type": "application/json" } })));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByText("Live best-effort positions")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    expect(screen.getByText("Received")).toBeInTheDocument();
    expect(screen.getByText("Snapshot age")).toBeInTheDocument();
    expect(screen.getByText("Provider state")).toBeInTheDocument();
    expect(screen.getAllByText("Observed trail").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Estimated 5-min projection/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Starts after next refresh/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/dashed projection is a geometric estimate/i)).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("shows independently toggleable NOAA layers and selectable METAR evidence", async () => {
    const user = userEvent.setup();
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      return Promise.resolve(new Response(JSON.stringify(
        url.includes("/weather") ? weatherPayload() : livePayload(),
      ), { status: 200, headers: { "Content-Type": "application/json" } }));
    }));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByText("Current NOAA")).toBeInTheDocument();
    const metarToggle = screen.getByRole("checkbox", { name: /Airports \/ METAR 1/i });
    const hazardToggle = screen.getByRole("checkbox", { name: /SIGMET hazards 1/i });
    expect(metarToggle).toBeChecked();
    expect(hazardToggle).toBeChecked();

    await user.selectOptions(screen.getByRole("combobox", { name: /Inspect weather evidence/i }), "observation:observation-1");
    expect(screen.getByRole("heading", { name: "KSFO · VFR" })).toBeInTheDocument();
    expect(screen.getByText("280° true · 15 kt")).toBeInTheDocument();

    await user.click(metarToggle);
    expect(metarToggle).not.toBeChecked();
    expect(screen.queryByRole("heading", { name: "KSFO · VFR" })).not.toBeInTheDocument();
    vi.unstubAllGlobals();
  });
});

function livePayload() {
  return {
    status: {
      enabled: true, provider: "adsb.lol", state: "current", best_effort: true,
      observed_at: "2026-07-21T23:00:00Z", last_success_at: "2026-07-21T23:00:00Z",
      newest_position_at: "2026-07-21T22:59:55Z", aircraft_count: 1,
      fresh_position_count: 1, stale_position_count: 0, stale_after_seconds: 300,
      region: { latitude_degrees: 37.62, longitude_degrees: -122.38, radius_nautical_miles: 50 },
      attribution: null,
    },
    data: [{
      id: "aircraft-1", callsign: "UAL123", aircraft_registration: null,
      longitude_degrees: -122.2, latitude_degrees: 37.6,
      altitude: { value: 12000, unit: "feet", reference: "ellipsoid" },
      heading_true_degrees: 270, ground_speed: { value: 310, unit: "knots" },
      quality: "observed", observed_at: new Date().toISOString(), received_at: new Date().toISOString(),
      provider: "adsb.lol",
    }],
  };
}

function weatherPayload() {
  return {
    state: "current", generated_at: "2026-07-21T23:00:00Z",
    attribution: { text: "Weather data from NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
    sources: [{
      provider: "noaa-awc", feed: "metar", state: "healthy",
      observed_at: "2026-07-21T23:00:00Z", last_success_at: "2026-07-21T23:00:00Z",
      newest_event_at: "2026-07-21T22:55:00Z", stale_after_seconds: 900, last_error_code: null,
    }],
    hazards: [{
      id: "hazard-1", source: { provider: "noaa-awc", feed: "airsigmet" }, status: "active",
      issued_at: "2026-07-21T22:30:00Z", hazard_type: "convective", severity: "significant",
      valid_from: "2026-07-21T22:30:00Z", valid_to: "2026-07-22T00:30:00Z", altitude_band: null,
      footprint: { exterior: [
        { longitude_degrees: -123, latitude_degrees: 37 }, { longitude_degrees: -121, latitude_degrees: 37 },
        { longitude_degrees: -121, latitude_degrees: 39 }, { longitude_degrees: -123, latitude_degrees: 37 },
      ] },
    }],
    observations: [{
      id: "observation-1", source: { provider: "noaa-awc", feed: "metar" },
      observed_at: "2026-07-21T22:55:00Z", received_at: "2026-07-21T22:56:00Z",
      station_code: "KSFO", report_type: "METAR",
      point: { longitude_degrees: -122.375, latitude_degrees: 37.619 },
      wind_direction_true_degrees: 280, wind_speed: { value: 15, unit: "knots" }, wind_gust: null,
      visibility_statute_miles: 10, visibility_greater_than: false, ceiling: null, flight_category: "visual",
    }],
  };
}
