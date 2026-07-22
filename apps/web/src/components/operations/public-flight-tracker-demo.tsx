"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  estimateTrajectory,
  type EstimatedTrajectory,
  type TrajectoryHistory,
  type TrajectoryPoint,
  updateTrajectoryHistory,
} from "@/lib/flight-trajectories";
import {
  parsePublicAttentionPicture,
  type PublicAircraftAttention,
  type PublicAttentionPicture,
} from "@/lib/public-attention";
import { PUBLIC_DEMO_FLIGHTS } from "@/lib/public-demo-data";
import {
  parsePublicLiveSnapshot,
  type PublicAircraft,
  type PublicLiveSnapshot,
} from "@/lib/public-live-positions";
import {
  parsePublicReplayTimeline,
  replayPictureAt,
  replayTrailAt,
  type PublicReplayTimeline,
} from "@/lib/public-replay-timeline";
import {
  DEFAULT_PUBLIC_LIVE_REGION,
  PUBLIC_LIVE_REGIONS,
  findPublicLiveRegion,
} from "@/lib/public-live-regions";
import {
  parsePublicWeatherSnapshot,
  type PublicWeatherSnapshot,
} from "@/lib/public-weather";
import { displayCallsign, LiveTrackerMap } from "./live-tracker-map";
import { ReplayTimeMachine } from "./replay-time-machine";

const POLL_INTERVAL_MS = 75_000;
const WEATHER_POLL_INTERVAL_MS = 60_000;
type TrackerMode = "live" | "stale" | "replay";
const PORTFOLIO_REPLAY_REGION = { ...DEFAULT_PUBLIC_LIVE_REGION, airport: "DEMO", name: "Portfolio replay" } as const;

export function PublicFlightTrackerDemo() {
  const [region, setRegion] = useState(DEFAULT_PUBLIC_LIVE_REGION);
  const [snapshot, setSnapshot] = useState<PublicLiveSnapshot | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshFailed, setRefreshFailed] = useState(false);
  const [trajectoryHistory, setTrajectoryHistory] = useState<TrajectoryHistory>(() => new Map());
  const [weather, setWeather] = useState<PublicWeatherSnapshot | null>(null);
  const [weatherLoading, setWeatherLoading] = useState(true);
  const [weatherRefreshFailed, setWeatherRefreshFailed] = useState(false);
  const [attentionPicture, setAttentionPicture] = useState<PublicAttentionPicture | null>(null);
  const [attentionLoading, setAttentionLoading] = useState(true);
  const [attentionFailed, setAttentionFailed] = useState(false);
  const [preferReplay, setPreferReplay] = useState(false);
  const [replayTimeline, setReplayTimeline] = useState<PublicReplayTimeline | null>(null);
  const [replayTimelineLoading, setReplayTimelineLoading] = useState(true);
  const [replayTimelineFailed, setReplayTimelineFailed] = useState(false);
  const [replayElapsedMs, setReplayElapsedMs] = useState(60_000);
  const [replayPlaying, setReplayPlaying] = useState(false);
  const [replaySpeed, setReplaySpeed] = useState(1);
  const fallbackAircraftForRegion = useMemo(() => replayAircraft(region.center), [region]);
  const replayPicture = useMemo(
    () => replayTimeline ? replayPictureAt(replayTimeline, replayElapsedMs) : null,
    [replayElapsedMs, replayTimeline],
  );
  const portfolioReplayAircraft = useMemo(
    () => (replayPicture?.aircraft ?? replayAircraft()).filter((item) => item.callsign !== "FT202"),
    [replayPicture],
  );

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch(`/api/public/live-positions?region=${region.code}`, { cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const next = parsePublicLiveSnapshot(await response.json());
      if (next.region_code !== region.code) throw new Error("Live tracker returned the wrong region");
      setSnapshot(next);
      setTrajectoryHistory((current) => updateTrajectoryHistory(current, next.data, Date.now()));
      setRefreshFailed(false);
      if (next.data.length > 0) {
        setSelectedId((current) => current && next.data.some((item) => item.id === current)
          ? current
          : next.data[0].id);
      }
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      setRefreshFailed(true);
    } finally {
      setLoading(false);
    }
  }, [region.code]);

  const refreshWeather = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch("/api/public/weather", { cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setWeather(parsePublicWeatherSnapshot(await response.json()));
      setWeatherRefreshFailed(false);
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      setWeatherRefreshFailed(true);
    } finally {
      setWeatherLoading(false);
    }
  }, []);

  const refreshAttention = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch("/api/public/replay/attention", { cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setAttentionPicture(parsePublicAttentionPicture(await response.json()));
      setAttentionFailed(false);
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      setAttentionFailed(true);
    } finally {
      setAttentionLoading(false);
    }
  }, []);

  const refreshReplayTimeline = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch("/api/public/replay/timeline", { cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setReplayTimeline(parsePublicReplayTimeline(await response.json()));
      setReplayTimelineFailed(false);
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      setReplayTimelineFailed(true);
    } finally {
      setReplayTimelineLoading(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    const initial = window.setTimeout(() => void refresh(controller.signal), 0);
    const timer = window.setInterval(() => void refresh(controller.signal), POLL_INTERVAL_MS);
    return () => {
      controller.abort();
      window.clearTimeout(initial);
      window.clearInterval(timer);
    };
  }, [refresh]);

  useEffect(() => {
    const controller = new AbortController();
    const initial = window.setTimeout(() => void refreshWeather(controller.signal), 0);
    const timer = window.setInterval(() => void refreshWeather(controller.signal), WEATHER_POLL_INTERVAL_MS);
    return () => {
      controller.abort();
      window.clearTimeout(initial);
      window.clearInterval(timer);
    };
  }, [refreshWeather]);

  useEffect(() => {
    const controller = new AbortController();
    const initial = window.setTimeout(() => void refreshAttention(controller.signal), 0);
    return () => {
      controller.abort();
      window.clearTimeout(initial);
    };
  }, [refreshAttention]);

  useEffect(() => {
    const controller = new AbortController();
    const initial = window.setTimeout(() => void refreshReplayTimeline(controller.signal), 0);
    return () => {
      controller.abort();
      window.clearTimeout(initial);
    };
  }, [refreshReplayTimeline]);

  useEffect(() => {
    if (!preferReplay || !replayPlaying || !replayTimeline) return;
    const tickMs = 100;
    const timer = window.setInterval(() => {
      setReplayElapsedMs((current) => {
        const next = Math.min(replayTimeline.duration_ms, current + tickMs * replaySpeed);
        if (next >= replayTimeline.duration_ms) setReplayPlaying(false);
        return next;
      });
    }, tickMs);
    return () => window.clearInterval(timer);
  }, [preferReplay, replayPlaying, replaySpeed, replayTimeline]);

  const hasAcceptedLivePicture = (snapshot?.data.length ?? 0) > 0;
  const fallbackReplay = !hasAcceptedLivePicture && !loading;
  const useReplay = preferReplay || fallbackReplay;
  const aircraft = preferReplay
    ? portfolioReplayAircraft
    : fallbackReplay
      ? fallbackAircraftForRegion
      : snapshot?.data ?? [];
  const displayedRegion = preferReplay ? PORTFOLIO_REPLAY_REGION : region;
  const mode: TrackerMode = useReplay ? "replay" : refreshFailed || snapshot?.status.state !== "current" ? "stale" : "live";
  const sourceState = loading ? "connecting" : snapshot?.status.state ?? (refreshFailed ? "unavailable" : "connecting");
  const selected = aircraft.find((item) => item.id === selectedId)
    ?? (preferReplay ? aircraft.find((item) => item.callsign === "FT303") : null)
    ?? aircraft[0]
    ?? null;
  const attentionEffectiveMs = replayTimeline && attentionPicture
    ? Date.parse(attentionPicture.scenario_time) - Date.parse(replayTimeline.start_time)
    : 60_000;
  const selectedAttention = preferReplay && replayElapsedMs >= attentionEffectiveMs && selected
    ? attentionPicture?.aircraft.find((item) => item.callsign === selected.callsign) ?? null
    : null;
  const selectedTrail = selected && preferReplay && replayTimeline
    ? replayTrailAt(replayTimeline, selected.callsign ?? "", replayElapsedMs, selected)
    : selected && !useReplay
      ? trajectoryHistory.get(selected.id) ?? []
      : [];
  const selectedProjection = selected && !useReplay ? estimateTrajectory(selected) : null;
  const weatherState = weather?.state ?? (weatherLoading ? "loading" : "unavailable");

  function handleRegionChange(code: string) {
    const nextRegion = findPublicLiveRegion(code);
    if (!nextRegion || nextRegion.code === region.code) return;
    setRegion(nextRegion);
    setSnapshot(null);
    setSelectedId(null);
    setTrajectoryHistory(new Map());
    setLoading(true);
    setRefreshFailed(false);
  }

  function handlePictureMode(nextMode: "live" | "replay") {
    const replay = nextMode === "replay";
    setPreferReplay(replay);
    setReplayPlaying(false);
    if (replay) setReplayElapsedMs(Math.max(0, Math.min(replayTimeline?.duration_ms ?? 60_000, attentionEffectiveMs)));
    setSelectedId(replay
      ? portfolioReplayAircraft.find((item) => item.callsign === "FT303")?.id ?? null
      : snapshot?.data[0]?.id ?? null);
  }

  function handleReplayElapsedChange(value: number) {
    setReplayPlaying(false);
    setReplayElapsedMs(Math.max(0, Math.min(replayTimeline?.duration_ms ?? value, value)));
  }

  return (
    <main className="operations-shell live-tracker-shell">
      <a className="skip-link" href="#live-flight-list">Skip to aircraft list</a>
      <header className="operations-header">
        <div className="product-lockup">
          <span className="product-mark" aria-hidden="true"><i /><i /><i /></span>
          <div>
            <p>Flight Tracker AI</p>
            <span>Realtime regional aircraft explorer</span>
          </div>
        </div>
        <div className="operations-summary" aria-label="Traffic summary">
          <SummaryMetric label="Tracked" value={loading ? "—" : String(aircraft.length)} />
          <SummaryMetric label="Fresh" value={useReplay ? "Demo" : String(snapshot?.status.fresh_position_count ?? 0)} />
          <SummaryMetric label="Region" value={preferReplay ? "Demo" : region.airport} />
          <SummaryMetric label="Refresh" value="75s" />
        </div>
        <div className="operations-controls">
          <label className="live-region-control">
            <span>Traffic region</span>
            <select
              aria-label="Live traffic region"
              value={region.code}
              disabled={preferReplay}
              onChange={(event) => handleRegionChange(event.target.value)}
            >
              {PUBLIC_LIVE_REGIONS.map((option) => (
                <option key={option.code} value={option.code}>
                  {option.airport} · {option.name}
                </option>
              ))}
            </select>
          </label>
          <div className="picture-mode-control" role="group" aria-label="Traffic source">
            <button type="button" aria-pressed={!preferReplay} onClick={() => handlePictureMode("live")}>Live traffic</button>
            <button type="button" aria-pressed={preferReplay} onClick={() => handlePictureMode("replay")}>Replay demo</button>
          </div>
          <span className={`phase ${mode === "live" ? "phase-active" : "phase-watch"}`}>
            {preferReplay ? "Deterministic replay scenario" : loading ? "Connecting to live traffic" : mode === "live" ? "Live best-effort positions" : mode === "stale" ? "Live source degraded" : "Deterministic replay fallback"}
          </span>
        </div>
      </header>

      {(sourceState !== "current" || useReplay) && (
        <div className={`live-state-banner state-${mode}`} role="status">
          <span>
            {preferReplay
              ? "Viewing the deterministic portfolio scenario. Replay facts and rule results are separate from live ADS-B positions."
              : loading
              ? "Connecting to ADSB.lol…"
              : sourceState === "disabled"
                ? "Live traffic is disabled, so the map is showing a clearly labeled replay demonstration."
                : mode === "replay"
                  ? `Live traffic is ${sourceState}, so the map is showing a clearly labeled replay demonstration.`
                  : `The provider is ${sourceState}; the last accepted live picture is retained while it reconnects.`}
          </span>
          {!loading && !preferReplay && <button type="button" onClick={() => void refresh()}>Try live again</button>}
        </div>
      )}

      {preferReplay && (
        <ReplayTimeMachine
          timeline={replayTimeline}
          loading={replayTimelineLoading}
          failed={replayTimelineFailed}
          elapsedMs={replayElapsedMs}
          playing={replayPlaying}
          speed={replaySpeed}
          selectedAircraft={selected}
          onElapsedChange={handleReplayElapsedChange}
          onPlayingChange={setReplayPlaying}
          onRestart={() => {
            setReplayPlaying(false);
            setReplayElapsedMs(0);
          }}
          onSpeedChange={setReplaySpeed}
          onRetry={() => void refreshReplayTimeline()}
        />
      )}

      <div className="live-tracker-grid">
        <LiveTrackerMap
          aircraft={aircraft}
          region={displayedRegion}
          selectedId={selected?.id ?? null}
          status={snapshot?.status ?? null}
          mode={mode}
          trail={selectedTrail}
          projection={selectedProjection}
          weather={weather}
          weatherState={weatherState}
          weatherRetained={weatherRefreshFailed && weather !== null}
          onRetryWeather={() => void refreshWeather()}
          onSelect={setSelectedId}
        />
        <div className="live-tracker-sidebar">
          <AircraftInspector
            aircraft={selected}
            mode={mode}
            status={snapshot?.status ?? null}
            trail={selectedTrail}
            projection={selectedProjection}
            attention={selectedAttention}
            attentionLoading={attentionLoading}
            attentionFailed={attentionFailed}
            onRetryAttention={() => void refreshAttention()}
          />
          <section className="ops-panel live-traffic-panel" id="live-flight-list" aria-labelledby="traffic-title">
            <div className="ops-panel-heading">
              <div><p className="ops-eyebrow">Current picture</p><h2 id="traffic-title">Aircraft</h2></div>
              <span className="traffic-count">{aircraft.length}</span>
            </div>
            <div className="live-aircraft-list">
              {aircraft.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className={item.id === selected?.id ? "live-flight-row is-selected" : "live-flight-row"}
                  onClick={() => setSelectedId(item.id)}
                >
                  <strong>{displayCallsign(item)}</strong>
                  <span>{formatAltitude(item)} · {formatSpeed(item)}</span>
                  <time dateTime={item.observed_at}>
                    {useReplay
                      ? `Replay · ${formatScenarioTime(item.observed_at)}`
                      : <>{isAircraftStale(item, snapshot?.status ?? null) ? "Stale · " : ""}{formatAge(item.observed_at)}</>}
                  </time>
                </button>
              ))}
              {!loading && aircraft.length === 0 && <p className="empty-traffic">No aircraft are visible in this regional snapshot.</p>}
            </div>
          </section>
        </div>
      </div>

      <footer className="operations-footer live-footer">
        <span>{mode === "replay" ? "Deterministic portfolio replay" : "Best-effort ADS-B positions from ADSB.lol"}</span>
        <span>Map © OpenFreeMap · OpenMapTiles · OpenStreetMap contributors</span>
        <span>UTC / WGS84 · Not for navigation</span>
      </footer>
    </main>
  );
}

function AircraftInspector({
  aircraft,
  mode,
  status,
  trail,
  projection,
  attention,
  attentionLoading,
  attentionFailed,
  onRetryAttention,
}: {
  aircraft: PublicAircraft | null;
  mode: TrackerMode;
  status: PublicLiveSnapshot["status"] | null;
  trail: readonly TrajectoryPoint[];
  projection: EstimatedTrajectory | null;
  attention: PublicAircraftAttention | null;
  attentionLoading: boolean;
  attentionFailed: boolean;
  onRetryAttention: () => void;
}) {
  return (
    <section className="ops-panel live-inspector" aria-labelledby="aircraft-detail-title">
      <div className="ops-panel-heading">
        <div><p className="ops-eyebrow">Selected aircraft</p><h2 id="aircraft-detail-title">{aircraft ? displayCallsign(aircraft) : "None selected"}</h2></div>
        <span className={`source-badge source-${mode}`}>{mode}</span>
      </div>
      <AttentionExplanation
        aircraft={aircraft}
        mode={mode}
        attention={attention}
        loading={attentionLoading}
        failed={attentionFailed}
        onRetry={onRetryAttention}
      />
      {aircraft ? (
        <dl className="aircraft-facts">
          <Fact label="Altitude" value={formatAltitude(aircraft)} />
          <Fact label="Ground speed" value={formatSpeed(aircraft)} />
          <Fact label="Heading" value={aircraft.heading_true_degrees == null ? "Not supplied" : `${Math.round(aircraft.heading_true_degrees)}° true`} />
          <Fact label="Position" value={`${aircraft.latitude_degrees.toFixed(4)}, ${aircraft.longitude_degrees.toFixed(4)}`} />
          <Fact label={mode === "replay" ? "Position time" : "Observed"} value={formatTimestamp(aircraft.observed_at)} />
          <Fact label={mode === "replay" ? "Last source fact" : "Received"} value={formatTimestamp(aircraft.received_at)} />
          <Fact
            label={mode === "replay" ? "Scenario time" : "Snapshot age"}
            value={mode === "replay" ? formatTimestamp(aircraft.observed_at) : formatAge(aircraft.observed_at)}
          />
          <Fact label="Freshness" value={mode === "replay" ? "Simulated" : isAircraftStale(aircraft, status) ? "Stale" : "Fresh"} />
          <Fact label="Provider state" value={mode === "replay" ? "Deterministic replay" : status?.state ?? "Connecting"} />
          <Fact label="Source quality" value={mode === "replay" ? aircraft.quality === "estimated" ? "Visual interpolation" : "Observed replay point" : aircraft.quality} />
          <Fact
            label="Observed trail"
            value={mode === "replay" ? trail.length === 0 ? "No history yet" : `${trail.length} replay points` : trail.length < 2 ? "Starts after next refresh" : `${trail.length} source points`}
          />
          <Fact
            label="Estimated projection"
            value={projection ? `${projection.horizon_minutes} min · ${projection.distance_nautical_miles.toFixed(1)} NM` : "Not available"}
          />
        </dl>
      ) : <p className="empty-traffic">Choose an aircraft to inspect its supplied position and motion facts.</p>}
      <p className="truth-note">
        {mode === "replay"
          ? "Replay points are deterministic scenario facts. Between source points, marker motion and telemetry are labeled visual interpolation—not a new observation, filed route, destination prediction, or ETA."
          : "The solid trail contains accepted observations. The dashed projection is a geometric estimate from current heading and speed—not a filed route, destination, ETA, or new source observation."}
      </p>
    </section>
  );
}

function AttentionExplanation({
  aircraft,
  mode,
  attention,
  loading,
  failed,
  onRetry,
}: {
  aircraft: PublicAircraft | null;
  mode: TrackerMode;
  attention: PublicAircraftAttention | null;
  loading: boolean;
  failed: boolean;
  onRetry: () => void;
}) {
  if (!aircraft) return null;
  if (mode !== "replay") {
    return (
      <section className="attention-explanation attention-unavailable" aria-labelledby="attention-title">
        <div className="attention-heading">
          <div><p className="ops-eyebrow">Decision intelligence</p><h3 id="attention-title">Not evaluated</h3></div>
          <span className="attention-state">Live position only</span>
        </div>
        <p>Live ADS-B supplies position and motion, but not the route and hazard evidence required for this deterministic assessment.</p>
      </section>
    );
  }
  if (loading) {
    return <section className="attention-explanation" aria-live="polite"><p>Evaluating the deterministic replay evidence…</p></section>;
  }
  if (failed) {
    return (
      <section className="attention-explanation attention-unavailable" aria-live="polite">
        <p>The replay explanation is unavailable. Aircraft and weather remain usable.</p>
        <button type="button" onClick={onRetry}>Try explanation again</button>
      </section>
    );
  }
  if (!attention || attention.state === "not_evaluated") {
    return (
      <section className="attention-explanation attention-unavailable" aria-labelledby="attention-title">
        <div className="attention-heading">
          <div><p className="ops-eyebrow">Decision intelligence</p><h3 id="attention-title">Not evaluated</h3></div>
          <span className="attention-state">Evidence incomplete</span>
        </div>
        <p>{attention?.summary ?? "No deterministic assessment is available for this replay aircraft."}</p>
        {attention?.source_times.flight_observed_at && <small>Replay position: {formatTimestamp(attention.source_times.flight_observed_at)}</small>}
      </section>
    );
  }

  const score = attention.score;
  const rule = attention.rule_result;
  const estimate = attention.geometric_estimate;
  if (!score || !rule || !estimate) return null;
  const scoreParts = [
    ["Hazard severity", score.hazard_severity_points],
    ["Route proximity", score.horizontal_proximity_points],
    ["Altitude overlap", score.altitude_overlap_points],
    ["Time urgency", score.time_urgency_points],
  ] as const;

  return (
    <section className="attention-explanation" aria-labelledby="attention-title" aria-live="polite">
      <div className="attention-heading">
        <div><p className="ops-eyebrow">Why this flight needs attention</p><h3 id="attention-title">{attention.priority} priority</h3></div>
        <strong className="attention-score" aria-label={`Attention score ${score.total} out of 100`}>{score.total}<small>/100</small></strong>
      </div>
      <p className="attention-summary">{attention.summary}</p>
      <div className="attention-score-parts" aria-label={`Score version ${score.score_version} breakdown`}>
        {scoreParts.map(([label, points]) => (
          <div key={label}><span>{label}</span><strong>+{points}</strong></div>
        ))}
      </div>
      <div className="attention-evidence-group">
        <h4>Replay facts</h4>
        <dl>{attention.observed_facts.map((fact) => <Fact key={fact.label} label={fact.label} value={fact.value} />)}</dl>
      </div>
      <div className="attention-evidence-group">
        <h4>Deterministic rule result</h4>
        <dl>
          <Fact label="Outcome" value={`${rule.horizontal_relation.replaceAll("_", " ")} · altitude ${rule.altitude_relation}`} />
          <Fact label="Policy" value={`${rule.rule_id} v${rule.rule_version} · score v${score.score_version}`} />
          <Fact label="Evidence versions" value={`Route v${rule.route_version} · hazard r${rule.hazard_revision}`} />
        </dl>
      </div>
      <div className="attention-evidence-group attention-estimate">
        <h4>Geometric estimate</h4>
        <p>Closest approach {estimate.closest_approach_nautical_miles.toFixed(1)} NM within a {estimate.proximity_margin_nautical_miles.toFixed(0)} NM rule margin.</p>
        <small>{estimate.disclaimer}</small>
      </div>
      <dl className="attention-times">
        <Fact label="Flight evidence" value={attention.source_times.flight_observed_at ? formatTimestamp(attention.source_times.flight_observed_at) : "Unavailable"} />
        <Fact label="Hazard issued" value={attention.source_times.hazard_issued_at ? formatTimestamp(attention.source_times.hazard_issued_at) : "Unavailable"} />
        <Fact label="Evaluated" value={formatTimestamp(attention.source_times.evaluated_at)} />
      </dl>
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return <div className="summary-metric"><span>{label}</span><strong>{value}</strong></div>;
}

function replayAircraft(center?: readonly [longitude: number, latitude: number]): PublicAircraft[] {
  const offsets = [[-0.42, -0.24], [0.08, 0.31], [0.46, -0.18]] as const;
  return PUBLIC_DEMO_FLIGHTS.flatMap((view, index) => view.latest_position ? [{
    id: view.flight.id,
    callsign: view.flight.callsign,
    aircraft_registration: view.flight.aircraft_registration,
    longitude_degrees: center ? center[0] + offsets[index][0] : view.latest_position.point.longitude_degrees,
    latitude_degrees: center ? center[1] + offsets[index][1] : view.latest_position.point.latitude_degrees,
    altitude: view.latest_position.altitude,
    heading_true_degrees: view.latest_position.heading_true_degrees,
    ground_speed: view.latest_position.ground_speed,
    quality: view.latest_position.quality,
    observed_at: view.latest_position.times.event_time,
    received_at: view.latest_position.times.received_at,
    provider: "portfolio.replay",
  }] : []);
}

function formatAltitude(item: PublicAircraft) {
  if (!item.altitude) return "Altitude unavailable";
  return `${item.altitude.value.toLocaleString()} ${item.altitude.unit === "feet" ? "ft" : "m"}`;
}

function formatSpeed(item: PublicAircraft) {
  if (!item.ground_speed) return "Speed unavailable";
  return `${Math.round(item.ground_speed.value)} ${item.ground_speed.unit === "knots" ? "kt" : "km/h"}`;
}

function formatAge(value: string) {
  const seconds = Math.max(0, Math.round((Date.now() - Date.parse(value)) / 1_000));
  return seconds < 60 ? `${seconds}s ago` : `${Math.floor(seconds / 60)}m ago`;
}

function formatTimestamp(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit",
    hour12: false, timeZone: "UTC", timeZoneName: "short",
  }).format(new Date(value));
}

function formatScenarioTime(value: string) {
  return new Intl.DateTimeFormat("en-US", {
    hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false, timeZone: "UTC",
  }).format(new Date(value)) + " UTC";
}

function isAircraftStale(item: PublicAircraft, status: PublicLiveSnapshot["status"] | null) {
  if (!status) return false;
  return Date.now() - Date.parse(item.observed_at) > status.stale_after_seconds * 1_000;
}
