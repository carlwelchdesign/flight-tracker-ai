"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  estimateTrajectory,
  type EstimatedTrajectory,
  type TrajectoryHistory,
  type TrajectoryPoint,
  updateTrajectoryHistory,
} from "@/lib/flight-trajectories";
import { PUBLIC_DEMO_FLIGHTS } from "@/lib/public-demo-data";
import {
  parsePublicLiveSnapshot,
  type PublicAircraft,
  type PublicLiveSnapshot,
} from "@/lib/public-live-positions";
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
import { PortfolioOrientation } from "./portfolio-orientation";

const POLL_INTERVAL_MS = 75_000;
const WEATHER_POLL_INTERVAL_MS = 60_000;
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
  const replayAircraftForRegion = useMemo(() => replayAircraft(region.center), [region]);

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

  const hasAcceptedLivePicture = (snapshot?.data.length ?? 0) > 0;
  const useReplay = !hasAcceptedLivePicture && !loading;
  const aircraft = useReplay ? replayAircraftForRegion : snapshot?.data ?? [];
  const mode = useReplay ? "replay" : refreshFailed || snapshot?.status.state !== "current" ? "stale" : "live";
  const sourceState = loading ? "connecting" : snapshot?.status.state ?? (refreshFailed ? "unavailable" : "connecting");
  const selected = aircraft.find((item) => item.id === selectedId) ?? aircraft[0] ?? null;
  const selectedTrail = selected && !useReplay ? trajectoryHistory.get(selected.id) ?? [] : [];
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

  return (
    <main className="operations-shell live-tracker-shell">
      <a className="skip-link" href="#live-flight-list">Skip to aircraft list</a>
      <PortfolioOrientation publicDemo />
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
          <SummaryMetric label="Region" value={region.airport} />
          <SummaryMetric label="Refresh" value="75s" />
        </div>
        <div className="operations-controls">
          <label className="live-region-control">
            <span>Traffic region</span>
            <select
              aria-label="Live traffic region"
              value={region.code}
              onChange={(event) => handleRegionChange(event.target.value)}
            >
              {PUBLIC_LIVE_REGIONS.map((option) => (
                <option key={option.code} value={option.code}>
                  {option.airport} · {option.name}
                </option>
              ))}
            </select>
          </label>
          <span className={`phase ${mode === "live" ? "phase-active" : "phase-watch"}`}>
            {loading ? "Connecting to live traffic" : mode === "live" ? "Live best-effort positions" : mode === "stale" ? "Live source degraded" : "Deterministic replay fallback"}
          </span>
          <Link href="/sign-in">Protected operations console</Link>
        </div>
      </header>

      {(sourceState !== "current" || useReplay) && (
        <div className={`live-state-banner state-${mode}`} role="status">
          <span>
            {loading
              ? "Connecting to ADSB.lol…"
              : sourceState === "disabled"
                ? "Live traffic is disabled, so the map is showing a clearly labeled replay demonstration."
                : mode === "replay"
                  ? `Live traffic is ${sourceState}, so the map is showing a clearly labeled replay demonstration.`
                  : `The provider is ${sourceState}; the last accepted live picture is retained while it reconnects.`}
          </span>
          {!loading && <button type="button" onClick={() => void refresh()}>Try live again</button>}
        </div>
      )}

      <div className="live-tracker-grid">
        <LiveTrackerMap
          aircraft={aircraft}
          region={region}
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
                    {isAircraftStale(item, snapshot?.status ?? null) ? "Stale · " : ""}{formatAge(item.observed_at)}
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
}: {
  aircraft: PublicAircraft | null;
  mode: string;
  status: PublicLiveSnapshot["status"] | null;
  trail: readonly TrajectoryPoint[];
  projection: EstimatedTrajectory | null;
}) {
  return (
    <section className="ops-panel live-inspector" aria-labelledby="aircraft-detail-title">
      <div className="ops-panel-heading">
        <div><p className="ops-eyebrow">Selected aircraft</p><h2 id="aircraft-detail-title">{aircraft ? displayCallsign(aircraft) : "None selected"}</h2></div>
        <span className={`source-badge source-${mode}`}>{mode}</span>
      </div>
      {aircraft ? (
        <dl className="aircraft-facts">
          <Fact label="Altitude" value={formatAltitude(aircraft)} />
          <Fact label="Ground speed" value={formatSpeed(aircraft)} />
          <Fact label="Heading" value={aircraft.heading_true_degrees == null ? "Not supplied" : `${Math.round(aircraft.heading_true_degrees)}° true`} />
          <Fact label="Position" value={`${aircraft.latitude_degrees.toFixed(4)}, ${aircraft.longitude_degrees.toFixed(4)}`} />
          <Fact label="Observed" value={formatTimestamp(aircraft.observed_at)} />
          <Fact label="Received" value={formatTimestamp(aircraft.received_at)} />
          <Fact label="Snapshot age" value={formatAge(aircraft.observed_at)} />
          <Fact label="Freshness" value={mode === "replay" ? "Simulated" : isAircraftStale(aircraft, status) ? "Stale" : "Fresh"} />
          <Fact label="Provider state" value={mode === "replay" ? "Replay fallback" : status?.state ?? "Connecting"} />
          <Fact label="Source quality" value={mode === "replay" ? "Simulated" : aircraft.quality} />
          <Fact
            label="Observed trail"
            value={mode === "replay" ? "Live only" : trail.length < 2 ? "Starts after next refresh" : `${trail.length} source points`}
          />
          <Fact
            label="Estimated projection"
            value={projection ? `${projection.horizon_minutes} min · ${projection.distance_nautical_miles.toFixed(1)} NM` : "Not available"}
          />
        </dl>
      ) : <p className="empty-traffic">Choose an aircraft to inspect its supplied position and motion facts.</p>}
      <p className="truth-note">
        The solid trail contains accepted observations. The dashed projection is a geometric estimate from current heading and speed—not a filed route, destination, ETA, or new source observation.
      </p>
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return <div className="summary-metric"><span>{label}</span><strong>{value}</strong></div>;
}

function replayAircraft(center: readonly [longitude: number, latitude: number]): PublicAircraft[] {
  const offsets = [[-0.42, -0.24], [0.08, 0.31], [0.46, -0.18]] as const;
  return PUBLIC_DEMO_FLIGHTS.flatMap((view, index) => view.latest_position ? [{
    id: view.flight.id,
    callsign: view.flight.callsign,
    aircraft_registration: view.flight.aircraft_registration,
    longitude_degrees: center[0] + offsets[index][0],
    latitude_degrees: center[1] + offsets[index][1],
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

function isAircraftStale(item: PublicAircraft, status: PublicLiveSnapshot["status"] | null) {
  if (!status) return false;
  return Date.now() - Date.parse(item.observed_at) > status.stale_after_seconds * 1_000;
}
