import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { replayPictureAt, type PublicReplayTimeline } from "@/lib/public-replay-timeline";
import { ReplayTimeMachine } from "./replay-time-machine";

describe("replay time machine", () => {
  it("offers accessible transport, scrubber, speed, and telemetry summaries", async () => {
    const user = userEvent.setup();
    const timeline = fixture();
    const aircraft = replayPictureAt(timeline, 60_000).aircraft[0];
    const onElapsedChange = vi.fn();
    const onPlayingChange = vi.fn();
    const onRestart = vi.fn();
    const onSpeedChange = vi.fn();

    render(
      <ReplayTimeMachine
        timeline={timeline}
        loading={false}
        failed={false}
        elapsedMs={60_000}
        playing={false}
        speed={1}
        selectedAircraft={aircraft}
        onElapsedChange={onElapsedChange}
        onPlayingChange={onPlayingChange}
        onRestart={onRestart}
        onSpeedChange={onSpeedChange}
        onRetry={vi.fn()}
      />,
    );

    expect(screen.getByRole("heading", { name: "16:01:00 UTC" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Paused · UTC scenario clock");
    expect(screen.getByRole("img", { name: /Altitude: 2 observations/i })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Ground speed: 2 observations/i })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: /Heading: 2 observations/i })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Play" }));
    expect(onPlayingChange).toHaveBeenCalledWith(true);
    await user.click(screen.getByRole("button", { name: "Restart" }));
    expect(onRestart).toHaveBeenCalled();
    await user.selectOptions(screen.getByRole("combobox", { name: "Replay speed" }), "2");
    expect(onSpeedChange).toHaveBeenCalledWith(2);
    fireEvent.change(screen.getByRole("slider", { name: "Replay scenario time" }), { target: { value: "30000" } });
    expect(onElapsedChange).toHaveBeenCalledWith(30_000);
  });

  it("describes no-history and unavailable states without hiding retry", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    const { rerender } = render(
      <ReplayTimeMachine
        timeline={fixture()}
        loading={false}
        failed={false}
        elapsedMs={0}
        playing={false}
        speed={1}
        selectedAircraft={null}
        onElapsedChange={vi.fn()}
        onPlayingChange={vi.fn()}
        onRestart={vi.fn()}
        onSpeedChange={vi.fn()}
        onRetry={onRetry}
      />,
    );
    expect(screen.getAllByText("No history at this scenario time")).toHaveLength(3);

    const singlePointAircraft = replayPictureAt(fixture(), 1_000).aircraft[0];
    rerender(
      <ReplayTimeMachine
        timeline={fixture()}
        loading={false}
        failed={false}
        elapsedMs={1_000}
        playing={false}
        speed={1}
        selectedAircraft={singlePointAircraft}
        onElapsedChange={vi.fn()}
        onPlayingChange={vi.fn()}
        onRestart={vi.fn()}
        onSpeedChange={vi.fn()}
        onRetry={onRetry}
      />,
    );
    expect(screen.getAllByText(/Single point/)).toHaveLength(3);

    rerender(
      <ReplayTimeMachine
        timeline={null}
        loading={false}
        failed
        elapsedMs={0}
        playing={false}
        speed={1}
        selectedAircraft={null}
        onElapsedChange={vi.fn()}
        onPlayingChange={vi.fn()}
        onRestart={vi.fn()}
        onSpeedChange={vi.fn()}
        onRetry={onRetry}
      />,
    );
    expect(screen.getByText(/static demonstration remains usable/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Try timeline again" }));
    expect(onRetry).toHaveBeenCalled();
  });
});

function fixture(): PublicReplayTimeline {
  return {
    schema_version: 1,
    scenario_id: "m1-operations-v1",
    start_time: "2026-07-20T16:00:00Z",
    end_time: "2026-07-20T16:03:00Z",
    duration_ms: 180_000,
    playback_speeds: [0.5, 1, 2],
    source: "portfolio deterministic replay",
    observations: [
      observation(1_000, 28_000, 445),
      observation(60_000, 27_000, 438),
    ],
  };
}

function observation(offsetMs: number, altitude: number, speed: number) {
  return {
    callsign: "FT303",
    aircraft_registration: "N303FT",
    offset_ms: offsetMs,
    observed_at: new Date(Date.parse("2026-07-20T16:00:00Z") + offsetMs).toISOString(),
    longitude_degrees: -121.72 + offsetMs / 600_000,
    latitude_degrees: 37 + offsetMs / 300_000,
    altitude: { value: altitude, unit: "feet" as const, reference: "mean_sea_level" },
    heading_true_degrees: 315,
    ground_speed: { value: speed, unit: "knots" as const },
    quality: "observed" as const,
  };
}
