import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PublicFlightTrackerDemo } from "./public-flight-tracker-demo";

describe("public flight tracker demo", () => {
  afterEach(() => {
    window.history.replaceState({}, "", "/");
  });
  it("shows the navigable map, replay fallback, and aircraft detail without an authentication prompt", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 503 })));
    const user = userEvent.setup();
    render(<PublicFlightTrackerDemo />);

    expect(screen.queryByLabelText("Portfolio use limitation")).not.toBeInTheDocument();
    expect(screen.queryByText(/recruiter walkthrough/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /see which flights need attention/i })).not.toBeInTheDocument();
    expect(screen.getByText("Flight Tracker AI")).toBeInTheDocument();
    expect(screen.getByText("Connecting to live traffic")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "None selected" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "San Francisco traffic" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Live traffic region" })).toHaveValue("sfo");
    expect(screen.getByRole("heading", { name: "Aircraft" })).toBeInTheDocument();
    expect(await screen.findByText(/replay demonstration/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "FT101" })).toBeInTheDocument();
    const selectedAircraft = screen.getByRole("heading", { name: "FT101" });
    const currentPicture = screen.getByRole("heading", { name: "Aircraft" });
    expect(selectedAircraft.compareDocumentPosition(currentPicture) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.queryByRole("link", { name: /protected operations console/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/sign in/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /FT303/i }));
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("links the public footer to Carl Welch's LinkedIn profile and the project repository", () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 503 })));
    render(<PublicFlightTrackerDemo />);

    expect(screen.getByRole("link", { name: "Carl Welch on LinkedIn" })).toHaveAttribute(
      "href",
      "https://www.linkedin.com/in/carlwelch",
    );
    expect(screen.getByRole("link", { name: "Flight Tracker AI source code on GitHub" })).toHaveAttribute(
      "href",
      "https://github.com/carlwelchdesign/flight-tracker-ai",
    );
    for (const link of screen.getAllByRole("link").filter((item) => item.closest(".portfolio-social-links"))) {
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).toHaveAttribute("rel", "noopener noreferrer");
      expect(link.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    }
    vi.unstubAllGlobals();
  });

  it("refreshes live traffic on the bounded 75-second polling cadence", async () => {
    const intervals: Array<{ callback: () => void; delay: number }> = [];
    vi.spyOn(window, "setInterval").mockImplementation(((callback: TimerHandler, delay?: number) => {
      if (typeof callback === "function") intervals.push({ callback: () => callback(), delay: delay ?? 0 });
      return intervals.length;
    }) as typeof window.setInterval);
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      const payload = url.includes("/weather")
        ? weatherPayload()
        : url.includes("/replay/attention")
          ? attentionPayload()
          : url.includes("/replay/timeline")
            ? replayTimelinePayload()
            : url.includes("/atmosphere/")
              ? windPayload()
              : livePayload();
      return Promise.resolve(new Response(JSON.stringify(payload), { status: 200 }));
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<PublicFlightTrackerDemo />);
    expect(await screen.findByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    const liveRequestsBefore = fetchMock.mock.calls.filter(([input]) => String(input).includes("/live-positions")).length;
    const trafficPoll = intervals.find(({ delay }) => delay === 75_000);
    expect(trafficPoll).toBeDefined();

    await act(async () => trafficPoll?.callback());
    await waitFor(() => {
      const liveRequestsAfter = fetchMock.mock.calls.filter(([input]) => String(input).includes("/live-positions")).length;
      expect(liveRequestsAfter).toBe(liveRequestsBefore + 1);
    });
    vi.unstubAllGlobals();
  });

  it("labels live source evidence and visual interpolation honestly", async () => {
    const payload = {
      region_code: "sfo",
      region_name: "San Francisco",
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
    };
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => Promise.resolve(
      String(input).includes("/live-positions")
        ? new Response(JSON.stringify(payload), { status: 200, headers: { "Content-Type": "application/json" } })
        : new Response(null, { status: 503 }),
    )));

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
        url.includes("/weather") ? weatherPayload() : url.includes("/atmosphere/") ? windPayload() : livePayload(),
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

  it("switches to another bounded live region without reloading the page", async () => {
    const user = userEvent.setup();
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/weather")) {
        return Promise.resolve(new Response(JSON.stringify(weatherPayload()), { status: 200 }));
      }
      if (url.includes("/atmosphere/")) {
        return Promise.resolve(new Response(JSON.stringify(windPayload()), { status: 200 }));
      }
      const isLosAngeles = url.includes("region=lax");
      return Promise.resolve(new Response(JSON.stringify(livePayload(
        isLosAngeles ? "lax" : "sfo",
        isLosAngeles ? "Los Angeles" : "San Francisco",
        isLosAngeles ? "AAL410" : "UAL123",
      )), { status: 200 }));
    });
    vi.stubGlobal("fetch", fetchMock);
    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    await user.selectOptions(screen.getByRole("combobox", { name: "Live traffic region" }), "lax");

    expect(await screen.findByRole("heading", { name: "Los Angeles traffic" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "AAL410" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "UAL123" })).not.toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/public/live-positions?region=lax",
      expect.objectContaining({ cache: "no-store" }),
    );
    vi.unstubAllGlobals();
  });

  it("hydrates a bounded replay link with aircraft, time, map, and weather state", async () => {
    window.history.replaceState({}, "", "/?mode=replay&scenario=m1-operations-v1&t=60000&aircraft=FT303&layers=metar%2Cradar%2Cmodel-wind&level=300&view=-121.6200%2C37.1800%2C8.25%2C-15.0%2C30.0");
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      const payload = url.includes("/replay/attention")
        ? attentionPayload()
        : url.includes("/replay/timeline")
          ? replayTimelinePayload()
          : url.includes("/weather")
            ? weatherPayload()
            : url.includes("/atmosphere/")
              ? windPayload()
              : livePayload();
      return Promise.resolve(new Response(JSON.stringify(payload), { status: 200 }));
    }));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByRole("heading", { name: "FT303" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Replay demo" })).toHaveAttribute("aria-pressed", "true");
    expect(await screen.findByRole("slider", { name: "Replay scenario time" })).toHaveValue("60000");
    expect(screen.getByRole("checkbox", { name: /Airports \/ METAR/i })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: /SIGMET hazards/i })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Radar" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Satellite clouds" })).not.toBeChecked();
    expect(screen.getByRole("combobox", { name: "Model wind level" })).toHaveValue("300");
    expect(window.location.search).toContain("aircraft=FT303");
    expect(window.location.search).toContain("view=-121.6200%2C37.1800%2C8.25%2C-15.0%2C30.0");
    vi.unstubAllGlobals();
  });

  it("filters the current picture and recovers from a shared aircraft that has expired", async () => {
    window.history.replaceState({}, "", "/?aircraft=UAL404");
    const user = userEvent.setup();
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      const payload = url.includes("/weather")
        ? weatherPayload()
        : url.includes("/atmosphere/")
          ? windPayload()
          : livePayload();
      return Promise.resolve(new Response(JSON.stringify(payload), { status: 200 }));
    }));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByText(/UAL404 is no longer in this snapshot/i)).toBeInTheDocument();
    expect(screen.getByText(/shared region and map view are still available/i)).toBeInTheDocument();
    const search = screen.getByRole("searchbox", { name: "Search this picture" });
    await user.type(search, "no match");
    expect(screen.getByText("0 of 1 aircraft")).toBeInTheDocument();
    expect(screen.getByText(/No callsign, ICAO hex, or registration matches/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Clear aircraft selection" }));
    expect(window.location.search).not.toContain("aircraft=");
    vi.unstubAllGlobals();
  });

  it("restores a bounded region when browser history changes", async () => {
    window.history.replaceState({}, "", "/?region=lax");
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/weather")) return Promise.resolve(new Response(JSON.stringify(weatherPayload()), { status: 200 }));
      if (url.includes("/atmosphere/")) return Promise.resolve(new Response(JSON.stringify(windPayload()), { status: 200 }));
      const lax = url.includes("region=lax");
      return Promise.resolve(new Response(JSON.stringify(livePayload(lax ? "lax" : "sfo", lax ? "Los Angeles" : "San Francisco", lax ? "AAL410" : "UAL123")), { status: 200 }));
    }));

    render(<PublicFlightTrackerDemo />);
    expect(await screen.findByRole("heading", { name: "Los Angeles traffic" })).toBeInTheDocument();

    window.history.replaceState({}, "", "/?region=sfo");
    window.dispatchEvent(new PopStateEvent("popstate"));
    expect(await screen.findByRole("heading", { name: "San Francisco traffic" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("switches from honest live non-evaluation to an explainable replay attention state", async () => {
    const user = userEvent.setup();
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      const payload = url.includes("/replay/attention")
        ? attentionPayload()
        : url.includes("/replay/timeline")
          ? replayTimelinePayload()
        : url.includes("/weather")
          ? weatherPayload()
          : url.includes("/atmosphere/")
            ? windPayload()
            : livePayload();
      return Promise.resolve(new Response(JSON.stringify(payload), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }));
    }));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Not evaluated" })).toBeInTheDocument();
    expect(screen.getByText("Live position only")).toBeInTheDocument();

    const replayButton = screen.getByRole("button", { name: "Replay demo" });
    replayButton.focus();
    expect(replayButton).toHaveFocus();
    await user.keyboard("{Enter}");

    expect(await screen.findByRole("heading", { name: "FT303" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "critical priority" })).toBeInTheDocument();
    expect(screen.getByLabelText("Attention score 85 out of 100")).toBeInTheDocument();
    expect(screen.getAllByText("27,000 ft").length).toBeGreaterThan(0);
    expect(screen.getByText("Hazard severity")).toBeInTheDocument();
    expect(screen.getByText(/route_hazard_proximity v1/i)).toBeInTheDocument();
    expect(screen.getByText(/not a filed route/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "16:01:00 UTC" })).toBeInTheDocument();
    expect(screen.getByText("2 replay points")).toBeInTheDocument();
    expect(screen.getByText("Replay trail")).toBeInTheDocument();
    expect(screen.getByText(/2 scenario points/i)).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Altitude: 2 observations/i })).toBeInTheDocument();

    fireEvent.change(screen.getByRole("slider", { name: "Replay scenario time" }), { target: { value: "0" } });
    expect(screen.getByRole("heading", { name: "None selected" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "critical priority" })).not.toBeInTheDocument();

    fireEvent.change(screen.getByRole("slider", { name: "Replay scenario time" }), { target: { value: "60000" } });
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "critical priority" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /FT101/i }));
    expect(screen.getByRole("heading", { name: "Not evaluated" })).toBeInTheDocument();
    expect(screen.getByText(/no route evidence/i)).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("keeps the replay usable when its explanation is unavailable", async () => {
    const user = userEvent.setup();
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/replay/attention")) return Promise.resolve(new Response(null, { status: 503 }));
      const payload = url.includes("/weather")
        ? weatherPayload()
        : url.includes("/atmosphere/")
          ? windPayload()
          : livePayload();
      return Promise.resolve(new Response(JSON.stringify(payload), { status: 200 }));
    }));

    render(<PublicFlightTrackerDemo />);
    expect(await screen.findByRole("heading", { name: "UAL123" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Replay demo" }));

    expect(await screen.findByText(/replay explanation is unavailable/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Try explanation again" })).toBeInTheDocument();
    vi.unstubAllGlobals();
  });

  it("retains a degraded live picture without inventing an assessment", async () => {
    const degraded = livePayload();
    degraded.status.state = "degraded";
    degraded.status.fresh_position_count = 0;
    degraded.status.stale_position_count = 1;
    vi.stubGlobal("fetch", vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      const payload = url.includes("/weather")
        ? weatherPayload()
        : url.includes("/atmosphere/")
          ? windPayload()
          : url.includes("/replay/attention")
            ? attentionPayload()
            : degraded;
      return Promise.resolve(new Response(JSON.stringify(payload), { status: 200 }));
    }));

    render(<PublicFlightTrackerDemo />);

    expect(await screen.findByText("Live source degraded")).toBeInTheDocument();
    expect(screen.getByText(/last accepted live picture is retained/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Not evaluated" })).toBeInTheDocument();
    expect(screen.getByText("Live position only")).toBeInTheDocument();
    vi.unstubAllGlobals();
  });
});

function livePayload(regionCode = "sfo", regionName = "San Francisco", callsign = "UAL123") {
  return {
    region_code: regionCode,
    region_name: regionName,
    status: {
      enabled: true, provider: "adsb.lol", state: "current", best_effort: true,
      observed_at: "2026-07-21T23:00:00Z", last_success_at: "2026-07-21T23:00:00Z",
      newest_position_at: "2026-07-21T22:59:55Z", aircraft_count: 1,
      fresh_position_count: 1, stale_position_count: 0, stale_after_seconds: 300,
      region: { latitude_degrees: 37.62, longitude_degrees: -122.38, radius_nautical_miles: 50 },
      attribution: null,
    },
    data: [{
      id: `aircraft-${regionCode}`, callsign, aircraft_registration: null,
      icao_hex: "A1B2C3",
      longitude_degrees: -122.2, latitude_degrees: 37.6,
      altitude: { value: 12000, unit: "feet", reference: "ellipsoid" },
      heading_true_degrees: 270, ground_speed: { value: 310, unit: "knots" },
      quality: "observed", observed_at: new Date().toISOString(), received_at: new Date().toISOString(),
      provider: "adsb.lol",
    }],
  };
}

function windPayload() {
  return {
    state: "current", retained: false, region_code: "sfo", region_name: "San Francisco",
    level: { code: "500", label: "500 hPa · about 18,000 ft", pressure_hpa: 500, approximate_altitude_feet: 18_400 },
    generated_at: "2026-07-21T23:00:00Z", forecast_time: "2026-07-21T23:00:00Z",
    last_success_at: "2026-07-21T23:00:00Z", last_error_code: null,
    attribution: {
      provider: "Open-Meteo", model: "NOAA GFS / HRRR",
      source_url: "https://open-meteo.com/",
      license_url: "https://open-meteo.com/en/license",
      text: "NOAA GFS/HRRR model data delivered by Open-Meteo",
    },
    samples: Array.from({ length: 16 }, (_, index) => ({
      latitude_degrees: 36.5 + Math.floor(index / 4) * 0.8,
      longitude_degrees: -123.5 + (index % 4) * 0.8,
      speed_knots: 35 + index,
      direction_from_degrees: 260 + index,
    })),
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

function attentionPayload() {
  const evaluatedAt = "2026-07-20T16:01:00Z";
  return {
    schema_version: 1,
    scenario_id: "m1-operations-v1",
    scenario_time: evaluatedAt,
    source: "portfolio deterministic replay",
    aircraft: [
      nonEvaluatedAttention("FT101", evaluatedAt),
      nonEvaluatedAttention("FT202", evaluatedAt),
      {
        callsign: "FT303",
        state: "requires_attention",
        priority: "critical",
        summary: "A significant convective hazard intersects the remaining replay route at the aircraft's demonstrated altitude.",
        observed_facts: [
          { label: "Replay route", value: "LAS to SFO · route version 1" },
          { label: "Aircraft altitude", value: "27000 feet" },
          { label: "Hazard evidence", value: "convective cell · significant · revision 1" },
        ],
        score: {
          hazard_severity_points: 45,
          horizontal_proximity_points: 25,
          altitude_overlap_points: 10,
          time_urgency_points: 5,
          total: 85,
          score_version: 1,
        },
        rule_result: {
          rule_id: "route_hazard_proximity",
          rule_version: 1,
          outcome: "match",
          route_version: 1,
          hazard_revision: 1,
          horizontal_relation: "intersects",
          altitude_relation: "overlap",
        },
        geometric_estimate: {
          closest_approach_nautical_miles: 0,
          proximity_margin_nautical_miles: 25,
          geometry_resolution_nautical_miles: 1,
          disclaimer: "Geometric rule estimate, not a filed route, clearance, destination prediction, or provider observation.",
        },
        source_times: {
          flight_observed_at: evaluatedAt,
          hazard_issued_at: "2026-07-20T16:00:00Z",
          evaluated_at: evaluatedAt,
        },
      },
    ],
  };
}

function replayTimelinePayload() {
  const start = Date.parse("2026-07-20T16:00:00Z");
  return {
    schema_version: 1,
    scenario_id: "m1-operations-v1",
    start_time: new Date(start).toISOString(),
    end_time: new Date(start + 180_000).toISOString(),
    duration_ms: 180_000,
    playback_speeds: [0.5, 1, 2],
    source: "portfolio deterministic replay",
    observations: [
      replayObservation("FT101", "N101FT", 1_000, -122.05, 37.42, 22_000, 142, 430),
      replayObservation("FT303", "N303FT", 1_000, -121.72, 37, 28_000, 315, 445),
      replayObservation("FT101", "N101FT", 60_000, -121.95, 37.25, 24_000, 142, 435),
      replayObservation("FT303", "N303FT", 60_000, -121.62, 37.18, 27_000, 315, 438),
      replayObservation("FT303", "N303FT", 180_000, -121.48, 37.38, 25_000, 318, 425),
    ],
  };
}

function replayObservation(
  callsign: string,
  registration: string,
  offsetMs: number,
  longitude: number,
  latitude: number,
  altitude: number,
  heading: number,
  speed: number,
) {
  return {
    callsign,
    aircraft_registration: registration,
    offset_ms: offsetMs,
    observed_at: new Date(Date.parse("2026-07-20T16:00:00Z") + offsetMs).toISOString(),
    longitude_degrees: longitude,
    latitude_degrees: latitude,
    altitude: { value: altitude, unit: "feet", reference: "mean_sea_level" },
    heading_true_degrees: heading,
    ground_speed: { value: speed, unit: "knots" },
    quality: "observed",
  };
}

function nonEvaluatedAttention(callsign: string, evaluatedAt: string) {
  return {
    callsign,
    state: "not_evaluated",
    priority: null,
    summary: "Not evaluated: this replay aircraft has no route evidence in the current scenario frame.",
    observed_facts: [{ label: "Replay position", value: evaluatedAt }],
    score: null,
    rule_result: null,
    geometric_estimate: null,
    source_times: {
      flight_observed_at: evaluatedAt,
      hazard_issued_at: null,
      evaluated_at: evaluatedAt,
    },
  };
}
