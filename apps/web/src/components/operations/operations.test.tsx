import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { FlightView } from "@/lib/fleet-api";
import type { AirportObservation, Hazard } from "@/lib/weather-api";
import { parseWeatherSnapshot } from "@/lib/weather-api";
import { parseFlightPage } from "@/lib/fleet-api";
import { FlightBoard } from "./flight-board";
import { FlightDetail } from "./flight-detail";
import { OperationsMap } from "./operations-map";
import {
  attentionLevel,
  fleetReferenceTime,
  fleetTiming,
  freshness,
  scheduleVariance,
} from "./operations-model";
import { OperationsBadges } from "./operations-badges";
import { OperationsStatusRegion } from "./operations-status";
import { hazardDisplayState } from "./weather-presentation";

const flights = [
  flight("101", "FT101", "SFO", "LAX", "active", -121.95, 37.25, "2026-07-20T16:01:00Z"),
  flight("202", "FT202", "SEA", "SFO", "scheduled", -122.3088, 47.4502, "2026-07-20T16:02:00Z"),
  flight("303", "FT303", "LAS", "SFO", "active", -121.62, 37.18, "2026-07-20T16:03:00Z"),
];

const hazards: Hazard[] = [
  {
    id: "hazard-1",
    operator_id: "00000000-0000-0000-0000-000000000999",
    schema_version: 1,
    source: {
      envelope_id: "00000000-0000-0000-0000-000000000901",
      provider: "noaa-awc",
      feed: "airsigmet",
      provider_record_id: "21W",
    },
    times: {
      event_time: "2026-07-20T16:00:00Z",
      received_at: "2026-07-20T16:00:05Z",
      processed_at: "2026-07-20T16:00:06Z",
    },
    external_series_id: "KKCI:21W:2026-07-20",
    revision: 2,
    supersedes_id: "00000000-0000-0000-0000-000000000900",
    status: "active",
    issued_at: "2026-07-20T16:00:00Z",
    provider_received_at: "2026-07-20T16:00:04Z",
    hazard_type: "convective_cell",
    severity: "significant",
    valid_from: "2026-07-20T16:00:00Z",
    valid_to: "2026-07-20T16:15:00Z",
    altitude_band: {
      lower: { value: 18_000, unit: "feet", reference: "flight_level" },
      upper: { value: 42_000, unit: "feet", reference: "flight_level" },
    },
    footprint: {
      exterior: [
        { longitude_degrees: -121.9, latitude_degrees: 37.1 },
        { longitude_degrees: -121.4, latitude_degrees: 37.1 },
        { longitude_degrees: -121.4, latitude_degrees: 37.5 },
        { longitude_degrees: -121.9, latitude_degrees: 37.5 },
        { longitude_degrees: -121.9, latitude_degrees: 37.1 },
      ],
    },
  },
];

const observations: AirportObservation[] = [{
  id: "observation-1",
  operator_id: "00000000-0000-0000-0000-000000000999",
  schema_version: 1,
  source: {
    envelope_id: "00000000-0000-0000-0000-000000000902",
    provider: "noaa-awc",
    feed: "metar",
    provider_record_id: "KSFO-1600",
  },
  times: {
    event_time: "2026-07-20T16:00:00Z",
    received_at: "2026-07-20T16:00:04Z",
    processed_at: "2026-07-20T16:00:05Z",
  },
  station_code: "KSFO",
  report_type: "METAR",
  raw_text: "METAR KSFO 201600Z 28012KT 10SM FEW010 18/13 A2992",
  provider_received_at: "2026-07-20T16:00:03Z",
  point: { longitude_degrees: -122.3656, latitude_degrees: 37.6196 },
  wind_direction_true_degrees: 280,
  wind_speed: { value: 12, unit: "knots" },
  wind_gust: null,
  visibility_statute_miles: 10,
  visibility_greater_than: true,
  ceiling: null,
  flight_category: "visual",
}];

describe("operations console interaction", () => {
  it("synchronizes map, board, and detail selection with accessible controls", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    expect(screen.getByRole("heading", { name: "FT101" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: /western united states route map/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /select ft303, watch attention/i })).toHaveStyle({
      "--aircraft-offset-y": "-38px",
    });

    await user.click(screen.getByRole("button", { name: /select ft303, watch attention/i }));
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /select flight ft303/i })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    const flight202 = screen.getByRole("button", { name: /select flight ft202/i });
    flight202.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByRole("heading", { name: "FT202" })).toBeInTheDocument();
    expect(screen.getAllByText("+32 min")).toHaveLength(2);
  });

  it("presents a recoverable empty state", () => {
    render(
      <FlightBoard
        flights={[]}
        hazards={[]}
        selectedId={null}
        refreshing={false}
        controlsAvailable={false}
        onSelect={() => undefined}
        onStart={() => undefined}
      />,
    );
    expect(screen.getByRole("heading", { name: "No active flight picture" })).toBeInTheDocument();
    expect(screen.getByText(/replay controls are unavailable/i)).toBeInTheDocument();
  });

  it("exposes weather layers, validity, altitude, source, and raw evidence", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    expect(screen.getByRole("checkbox", { name: /hazards 1/i })).toBeChecked();
    expect(screen.getByRole("img", { name: /ksfo vfr, current/i })).toBeInTheDocument();
    const hazard = screen.getByRole("button", {
      name: /convective_cell hazard, significant, fl180 – fl420, active/i,
    });
    hazard.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByRole("heading", { name: "convective cell" })).toBeInTheDocument();
    expect(screen.getByText("NOAA AWC · airsigmet")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /open raw noaa source/i })).toHaveAttribute(
      "href",
      "/api/backend/api/source-records/00000000-0000-0000-0000-000000000901",
    );

    await user.click(screen.getByRole("checkbox", { name: /hazards 1/i }));
    expect(screen.queryByRole("button", { name: /convective_cell hazard/i })).not.toBeInTheDocument();
  });

  it("retains accepted weather evidence when a refresh becomes unavailable", () => {
    render(
      <OperationsMap
        flights={[]}
        hazards={hazards}
        observations={observations}
        sourceHealth={[]}
        weatherState="unavailable"
        weatherMessage="Weather refresh returned HTTP 503"
        weatherAsOf="2026-07-20T16:03:00Z"
        selectedId={null}
        selectedHazardId={null}
        onSelect={() => undefined}
        onSelectHazard={() => undefined}
        onRetryWeather={() => undefined}
      />,
    );
    expect(screen.getByText(/retaining the last accepted layer/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /convective_cell hazard/i })).toBeInTheDocument();
  });
});

describe("operational presentation rules", () => {
  it("derives delayed and hazard-adjacent attention from source facts", () => {
    const reference = fleetReferenceTime(flights);
    expect(scheduleVariance(flights[1])).toEqual({ label: "+32 min", minutes: 32 });
    expect(attentionLevel(flights[1], hazards, reference)).toMatchObject({
      level: "watch",
      reason: "+32 min departure",
    });
    expect(attentionLevel(flights[2], hazards, reference)).toMatchObject({
      level: "watch",
      reason: "Hazard-adjacent track",
    });
  });

  it("makes stale source data visible and attention-worthy", () => {
    const reference = Date.parse("2026-07-20T16:03:01Z");
    expect(freshness(flights[0], reference)).toEqual({ level: "stale", label: "2m behind" });
    expect(attentionLevel(flights[0], [], reference)).toMatchObject({
      level: "watch",
      reason: "Position data is stale",
    });
  });

  it("rejects malformed fleet transport payloads before rendering", () => {
    expect(() => parseFlightPage({ data: [{}], pagination: {} })).toThrow(
      "Fleet API returned an unexpected list payload",
    );
  });

  it("keeps event and receipt timing distinct", () => {
    const delayedReceipt: FlightView = {
      ...flights[2],
      latest_position: {
        ...flights[2].latest_position!,
        times: {
          ...flights[2].latest_position!.times,
          received_at: "2026-07-20T16:04:30Z",
        },
      },
    };
    expect(fleetTiming([flights[0], flights[1], delayedReceipt])).toEqual({
      lastEventTime: Date.parse("2026-07-20T16:03:00Z"),
      lastReceivedTime: Date.parse("2026-07-20T16:04:30Z"),
    });
  });

  it("distinguishes expired and cancelled hazards and excludes them from attention", () => {
    expect(hazardDisplayState(hazards[0], Date.parse("2026-07-20T16:16:00Z"))).toBe("expired");
    const cancelled = { ...hazards[0], status: "cancelled" as const };
    expect(hazardDisplayState(cancelled, Date.parse("2026-07-20T16:03:00Z"))).toBe("cancelled");
    expect(attentionLevel(flights[2], [cancelled], fleetReferenceTime(flights))).toMatchObject({
      level: "normal",
    });
  });

  it("rejects malformed weather payloads before rendering", () => {
    expect(() => parseWeatherSnapshot(
      { data: [{ id: "unsafe" }], generated_at: "2026-07-20T16:03:00Z" },
      { data: [], generated_at: "2026-07-20T16:03:00Z" },
      { data: [] },
    )).toThrow("unexpected hazard");
  });
});

describe("operations health presentation", () => {
  it("distinguishes healthy service and stream state", () => {
    render(
      <OperationsBadges
        connection="live"
        serviceHealth={{ state: "healthy", workers: [] }}
      />,
    );
    expect(screen.getByText("Service healthy")).toBeInTheDocument();
    expect(screen.getByText("Stream live")).toBeInTheDocument();
  });

  it("makes a simulated feed outage obvious and recoverable", async () => {
    const restore = vi.fn();
    const user = userEvent.setup();
    render(
      <OperationsStatusRegion
        connection="live"
        serviceHealth={{ state: "healthy", workers: [] }}
        feedOutage
        error={null}
        onRetry={() => undefined}
        onDismiss={() => undefined}
        onRestoreFeed={restore}
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent("Simulation feed outage");
    await user.click(screen.getByRole("button", { name: "Restore feed" }));
    expect(restore).toHaveBeenCalledOnce();
  });

  it("names a failed critical worker when service health degrades", () => {
    render(
      <OperationsStatusRegion
        connection="live"
        serviceHealth={{
          state: "degraded",
          workers: [
            {
              name: "replay_runtime",
              state: "failed",
              last_heartbeat_at: null,
              detail: "scenario loop stopped",
            },
          ],
          message: "replay_runtime (failed)",
        }}
        feedOutage={false}
        error={null}
        onRetry={() => undefined}
        onDismiss={() => undefined}
        onRestoreFeed={() => undefined}
      />,
    );
    expect(screen.getByText("replay_runtime (failed)")).toBeInTheDocument();
  });
});

function Harness() {
  const [selectedId, setSelectedId] = useState(flights[0].flight.id);
  const [selectedHazardId, setSelectedHazardId] = useState<string | null>(null);
  const selected = flights.find((view) => view.flight.id === selectedId) ?? null;
  return (
    <>
      <OperationsMap
        flights={flights}
        hazards={hazards}
        observations={observations}
        sourceHealth={[]}
        weatherState="ready"
        weatherMessage={null}
        weatherAsOf="2026-07-20T16:03:00Z"
        selectedHazardId={selectedHazardId}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onSelectHazard={setSelectedHazardId}
        onRetryWeather={() => undefined}
      />
      <FlightBoard
        flights={flights}
        hazards={hazards}
        selectedId={selectedId}
        refreshing={false}
        controlsAvailable
        onSelect={setSelectedId}
        onStart={() => undefined}
      />
      <FlightDetail
        selected={selected}
        flights={flights}
        hazards={hazards}
        timeline={[]}
        timelineState="ready"
      />
    </>
  );
}

function flight(
  suffix: string,
  flightCallsign: string,
  origin: string,
  destination: string,
  status: FlightView["flight"]["status"],
  longitude: number,
  latitude: number,
  eventTime: string,
): FlightView {
  const envelopeId = `00000000-0000-0000-0000-000000000${suffix}`;
  const source = {
    envelope_id: envelopeId,
    provider: "simulation",
    feed: "m1-operations-v1",
    provider_record_id: `record-${suffix}`,
  };
  const times = { event_time: eventTime, received_at: eventTime, processed_at: eventTime };
  return {
    flight: {
      id: `00000000-0000-0000-0000-000000000${suffix}`,
      operator_id: "00000000-0000-0000-0000-000000000999",
      schema_version: 1,
      source,
      times,
      callsign: flightCallsign,
      aircraft_registration: `N${suffix}FT`,
      origin_airport_code: origin,
      destination_airport_code: destination,
      scheduled_departure_at: "2026-07-20T15:30:00Z",
      scheduled_arrival_at: "2026-07-20T17:30:00Z",
      status,
    },
    latest_position: {
      id: `10000000-0000-0000-0000-000000000${suffix}`,
      operator_id: "00000000-0000-0000-0000-000000000999",
      flight_id: `00000000-0000-0000-0000-000000000${suffix}`,
      schema_version: 1,
      source,
      times,
      point: { longitude_degrees: longitude, latitude_degrees: latitude },
      altitude: { value: 24_000, unit: "feet", reference: "mean_sea_level" },
      heading_true_degrees: 310,
      ground_speed: { value: 430, unit: "knots" },
      quality: "observed",
    },
  };
}
