"use client";

import type { PublicAircraft } from "@/lib/public-live-positions";
import type { PublicReplayObservation, PublicReplayTimeline } from "@/lib/public-replay-timeline";

type Props = {
  timeline: PublicReplayTimeline | null;
  loading: boolean;
  failed: boolean;
  elapsedMs: number;
  playing: boolean;
  speed: number;
  selectedAircraft: PublicAircraft | null;
  onElapsedChange: (elapsedMs: number) => void;
  onPlayingChange: (playing: boolean) => void;
  onRestart: () => void;
  onSpeedChange: (speed: number) => void;
  onRetry: () => void;
};

export function ReplayTimeMachine({
  timeline,
  loading,
  failed,
  elapsedMs,
  playing,
  speed,
  selectedAircraft,
  onElapsedChange,
  onPlayingChange,
  onRestart,
  onSpeedChange,
  onRetry,
}: Props) {
  if (loading) {
    return <section className="replay-time-machine replay-time-machine-state" aria-live="polite">Loading replay timeline…</section>;
  }
  if (failed || !timeline) {
    return (
      <section className="replay-time-machine replay-time-machine-state" aria-live="polite">
        <span>The replay timeline is unavailable. The static demonstration remains usable.</span>
        <button type="button" onClick={onRetry}>Try timeline again</button>
      </section>
    );
  }

  const complete = elapsedMs >= timeline.duration_ms;
  const scenarioTime = new Date(Date.parse(timeline.start_time) + elapsedMs);
  const callsign = selectedAircraft?.callsign ?? null;
  const observations = callsign
    ? timeline.observations.filter((observation) => observation.callsign === callsign && observation.offset_ms <= elapsedMs)
    : [];

  return (
    <section className="replay-time-machine" aria-labelledby="replay-time-machine-title">
      <div className="replay-control-row">
        <div>
          <p className="ops-eyebrow">Replay time machine</p>
          <h2 id="replay-time-machine-title">{formatScenarioTime(scenarioTime)}</h2>
        </div>
        <div className="replay-transport" role="group" aria-label="Replay transport controls">
          <button type="button" onClick={() => onPlayingChange(!playing)} disabled={complete && !playing}>
            {playing ? "Pause" : complete ? "Complete" : "Play"}
          </button>
          <button type="button" onClick={onRestart}>Restart</button>
          <label>
            <span>Speed</span>
            <select aria-label="Replay speed" value={speed} onChange={(event) => onSpeedChange(Number(event.target.value))}>
              {timeline.playback_speeds.map((value) => <option key={value} value={value}>{value}×</option>)}
            </select>
          </label>
        </div>
        <label className="replay-scrubber">
          <span className="sr-only">Replay scenario time</span>
          <input
            type="range"
            aria-label="Replay scenario time"
            min={0}
            max={timeline.duration_ms}
            step={1_000}
            value={Math.round(elapsedMs / 1_000) * 1_000}
            onChange={(event) => onElapsedChange(Number(event.target.value))}
          />
          <span>{formatElapsed(elapsedMs)} / {formatElapsed(timeline.duration_ms)}</span>
        </label>
        <div className="replay-clock-state" role="status">
          {complete ? "Replay complete" : playing ? `Playing at ${speed}×` : "Paused"} · UTC scenario clock
        </div>
      </div>
      <TelemetryCharts
        observations={observations}
        currentAircraft={selectedAircraft}
        elapsedMs={elapsedMs}
        durationMs={timeline.duration_ms}
      />
    </section>
  );
}

function TelemetryCharts({
  observations,
  currentAircraft,
  elapsedMs,
  durationMs,
}: {
  observations: PublicReplayObservation[];
  currentAircraft: PublicAircraft | null;
  elapsedMs: number;
  durationMs: number;
}) {
  const callsign = currentAircraft?.callsign ?? "Selected aircraft";
  const series = [
    {
      key: "altitude",
      label: "Altitude",
      unit: currentAircraft?.altitude?.unit === "meters" ? "m" : "ft",
      observed: observations.flatMap((item) => item.altitude ? [{ offsetMs: item.offset_ms, value: item.altitude.value }] : []),
      current: currentAircraft?.altitude?.value ?? null,
    },
    {
      key: "speed",
      label: "Ground speed",
      unit: currentAircraft?.ground_speed?.unit === "kilometers_per_hour" ? "km/h" : "kt",
      observed: observations.flatMap((item) => item.ground_speed ? [{ offsetMs: item.offset_ms, value: item.ground_speed.value }] : []),
      current: currentAircraft?.ground_speed?.value ?? null,
    },
    {
      key: "heading",
      label: "Heading",
      unit: "° true",
      observed: observations.flatMap((item) => item.heading_true_degrees == null ? [] : [{ offsetMs: item.offset_ms, value: item.heading_true_degrees }]),
      current: currentAircraft?.heading_true_degrees ?? null,
    },
  ] as const;

  return (
    <div className="telemetry-charts" aria-label={`${callsign} replay telemetry`}>
      {series.map((item) => (
        <TelemetryChart
          key={item.key}
          label={item.label}
          unit={item.unit}
          observed={item.observed}
          current={item.current}
          elapsedMs={elapsedMs}
          durationMs={durationMs}
        />
      ))}
    </div>
  );
}

function TelemetryChart({
  label,
  unit,
  observed,
  current,
  elapsedMs,
  durationMs,
}: {
  label: string;
  unit: string;
  observed: Array<{ offsetMs: number; value: number }>;
  current: number | null;
  elapsedMs: number;
  durationMs: number;
}) {
  const points = [...observed];
  if (current !== null && points.at(-1)?.offsetMs !== elapsedMs) points.push({ offsetMs: elapsedMs, value: current });
  const values = points.map((point) => point.value);
  const minimum = values.length > 0 ? Math.min(...values) : 0;
  const maximum = values.length > 0 ? Math.max(...values) : 0;
  const spread = Math.max(1, maximum - minimum);
  const polyline = points.map((point) => {
    const x = 8 + (point.offsetMs / durationMs) * 284;
    const y = 58 - ((point.value - minimum) / spread) * 46;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const summary = points.length === 0
    ? "No history at this scenario time"
    : points.length === 1
      ? `Single point · ${formatValue(points[0].value)} ${unit}`
      : `${observed.length} observations · ${formatValue(minimum)} to ${formatValue(maximum)} ${unit} · current ${formatValue(current ?? points.at(-1)!.value)} ${unit}`;

  return (
    <figure className="telemetry-chart">
      <figcaption><span>{label}</span><strong>{current === null ? "—" : `${formatValue(current)} ${unit}`}</strong></figcaption>
      <svg viewBox="0 0 300 66" role="img" aria-label={`${label}: ${summary}`} focusable="true">
        <path d="M8 58H292" className="telemetry-axis" />
        {polyline && <polyline points={polyline} className="telemetry-line" />}
        {points.map((point) => {
          const x = 8 + (point.offsetMs / durationMs) * 284;
          const y = 58 - ((point.value - minimum) / spread) * 46;
          return <circle key={`${point.offsetMs}-${point.value}`} cx={x} cy={y} r="2.5" className="telemetry-point" />;
        })}
      </svg>
      <p>{summary}</p>
    </figure>
  );
}

function formatValue(value: number) {
  return Math.round(value).toLocaleString();
}

function formatElapsed(value: number) {
  const totalSeconds = Math.floor(value / 1_000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function formatScenarioTime(value: Date) {
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone: "UTC",
    timeZoneName: "short",
  }).format(value);
}
