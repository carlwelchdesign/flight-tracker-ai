import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AirportIntelligencePanel } from "./airport-intelligence-panel";

afterEach(() => vi.unstubAllGlobals());

describe("AirportIntelligencePanel", () => {
  it("distinguishes TAF periods from nearby pilot reports and explains coverage", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(payload), { status: 200 })));
    render(<AirportIntelligencePanel airport="SFO" />);
    await userEvent.click(screen.getByRole("button", { name: /KSFO forecast/i }));
    expect(await screen.findByRole("heading", { name: "Terminal forecast" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Nearby pilot reports" })).toBeInTheDocument();
    expect(screen.getByText(/not attributed to any selected flight/i)).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith("/api/public/airport-intelligence?airport=KSFO", expect.anything());
  });

  it("keeps the rest of the tracker usable when airport intelligence fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));
    render(<AirportIntelligencePanel airport="SFO" />);
    await userEvent.click(screen.getByRole("button", { name: /KSFO forecast/i }));
    expect(await screen.findByText(/Aircraft and other weather layers remain usable/i)).toBeInTheDocument();
  });
});

const payload = {
  state: "current", generated_at: "2026-07-22T10:00:00Z",
  airport: { code: "KSFO", name: "San Francisco International", latitude_degrees: 37.62, longitude_degrees: -122.36 },
  attribution: { text: "NOAA Aviation Weather Center", source_url: "https://aviationweather.gov/" },
  taf: { state: "current", accepted_at: "2026-07-22T10:00:00Z", data: { issue_time: "2026-07-22T09:00:00Z", valid_from: "2026-07-22T09:00:00Z", valid_to: "2026-07-23T12:00:00Z", periods: [{ valid_from: "2026-07-22T09:00:00Z", valid_to: "2026-07-22T12:00:00Z", change: "TEMPO", probability_percent: 30, wind_direction_degrees: 220, wind_speed_knots: 15, wind_gust_knots: null, visibility: "6+", weather: null, clouds: [{ coverage: "BKN", base_feet_agl: 1200 }] }] } },
  pireps: { state: "current", accepted_at: "2026-07-22T10:00:00Z", data: [{ report_time: "2026-07-22T09:30:00Z", received_at: "2026-07-22T09:31:00Z", distance_nautical_miles: 12.4, altitude_feet: 7000, altitude_context: "OTHER", report_type: "PIREP", aircraft_type: "B737", turbulence: "LGT CHOP", icing: null, clouds: null, wind: null, temperature_celsius: null, weather: null, location_available: true }] },
  coverage_note: "Nearby pilot reports are sparse and not attributed to any selected flight.",
};
