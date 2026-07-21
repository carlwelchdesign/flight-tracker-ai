import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { LivePositionStatus } from "@/lib/live-positions-api";

import { LivePositionSource } from "./live-position-source";

const current: LivePositionStatus = {
  enabled: true,
  provider: "adsb.lol",
  feed: "point",
  state: "current",
  best_effort: true,
  observed_at: "2026-07-21T17:16:02Z",
  last_attempt_at: "2026-07-21T17:16:02Z",
  last_success_at: "2026-07-21T17:16:02Z",
  newest_position_at: "2026-07-21T17:16:00Z",
  consecutive_failures: 0,
  aircraft_count: 12,
  fresh_position_count: 10,
  stale_position_count: 2,
  rejected_record_count: 1,
  missing_callsign_count: 3,
  stale_after_seconds: 30,
  last_error_code: null,
  region: {
    latitude_degrees: 37.62,
    longitude_degrees: -122.38,
    radius_nautical_miles: 25,
  },
  attribution: {
    text: "Contains information from ADSB.lol, available under the Open Database License (ODbL).",
    source_url: "https://adsb.lol/",
    license_url: "https://opendatacommons.org/licenses/odbl/1-0/",
  },
};

describe("live position source presentation", () => {
  it("shows best-effort coverage, freshness, and linked ODbL attribution", () => {
    render(
      <LivePositionSource
        status={current}
        message={null}
        liveFlightsVisible
        replayAvailable
        onUseReplay={() => undefined}
        onRetry={() => undefined}
      />,
    );

    expect(screen.getByRole("heading", { name: "ADSB.lol positions available" })).toBeInTheDocument();
    expect(screen.getByText(/10 positions meet the 30-second freshness threshold/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "ADSB.lol" })).toHaveAttribute(
      "href",
      "https://adsb.lol/",
    );
    expect(screen.getByRole("link", { name: /open database license/i })).toHaveAttribute(
      "href",
      "https://opendatacommons.org/licenses/odbl/1-0/",
    );
  });

  it("makes failure explicit and offers the deterministic replay path", async () => {
    const useReplay = vi.fn();
    const retry = vi.fn();
    const user = userEvent.setup();
    render(
      <LivePositionSource
        status={{
          ...current,
          state: "unavailable",
          consecutive_failures: 3,
          last_error_code: "provider_unavailable",
        }}
        message={null}
        liveFlightsVisible
        replayAvailable
        onUseReplay={useReplay}
        onRetry={retry}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Live positions unavailable · replay preserved",
    );
    await user.click(screen.getByRole("button", { name: "Use replay view" }));
    await user.click(screen.getByRole("button", { name: "Retry status" }));
    expect(useReplay).toHaveBeenCalledOnce();
    expect(retry).toHaveBeenCalledOnce();
  });

  it("shows a high-latency timeout as degraded while replay stays selectable", async () => {
    const useReplay = vi.fn();
    const user = userEvent.setup();
    render(
      <LivePositionSource
        status={{
          ...current,
          state: "degraded",
          consecutive_failures: 1,
          last_error_code: "timeout",
        }}
        message={null}
        liveFlightsVisible
        replayAvailable
        onUseReplay={useReplay}
        onRetry={() => undefined}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(
      "Live positions degraded · replay preserved",
    );
    expect(screen.getByRole("status")).toHaveTextContent("Source condition: timeout");
    await user.click(screen.getByRole("button", { name: "Use replay view" }));
    expect(useReplay).toHaveBeenCalledOnce();
  });

  it("keeps replay as the complete default when the external layer is disabled", () => {
    render(
      <LivePositionSource
        status={{
          ...current,
          enabled: false,
          provider: null,
          feed: null,
          state: "disabled",
          region: null,
          attribution: null,
        }}
        message={null}
        liveFlightsVisible={false}
        replayAvailable
        onUseReplay={() => undefined}
        onRetry={() => undefined}
      />,
    );
    expect(screen.getByRole("heading", { name: /deterministic replay is the default/i })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "ADSB.lol" })).not.toBeInTheDocument();
  });
});
