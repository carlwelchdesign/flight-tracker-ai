"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { PUBLIC_DEMO_FLIGHTS } from "@/lib/public-demo-data";
import {
  parsePublicLiveSnapshot,
  type PublicAircraft,
  type PublicLiveSnapshot,
} from "@/lib/public-live-positions";
import { displayCallsign, LiveTrackerMap } from "./live-tracker-map";
import { PortfolioOrientation } from "./portfolio-orientation";

const POLL_INTERVAL_MS = 30_000;
const REPLAY_AIRCRAFT = replayAircraft();

export function PublicFlightTrackerDemo() {
  const [snapshot, setSnapshot] = useState<PublicLiveSnapshot | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshFailed, setRefreshFailed] = useState(false);

  const refresh = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch("/api/public/live-positions", { cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const next = parsePublicLiveSnapshot(await response.json());
      setSnapshot(next);
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

  const hasAcceptedLivePicture = (snapshot?.data.length ?? 0) > 0;
  const useReplay = !hasAcceptedLivePicture && !loading;
  const aircraft = useReplay ? REPLAY_AIRCRAFT : snapshot?.data ?? [];
  const mode = useReplay ? "replay" : refreshFailed || snapshot?.status.state !== "current" ? "stale" : "live";
  const selected = aircraft.find((item) => item.id === selectedId) ?? aircraft[0] ?? null;

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
          <SummaryMetric label="Region" value="SFO" />
          <SummaryMetric label="Refresh" value="30s" />
        </div>
        <div className="operations-controls">
          <span className={`phase ${mode === "live" ? "phase-active" : "phase-watch"}`}>
            {loading ? "Connecting to live traffic" : mode === "live" ? "Live best-effort positions" : mode === "stale" ? "Live source degraded" : "Deterministic replay fallback"}
          </span>
          <Link href="/sign-in">Protected operations console</Link>
        </div>
      </header>

      {(mode !== "live" || loading) && (
        <div className={`live-state-banner state-${mode}`} role="status">
          <span>
            {loading
              ? "Connecting to ADSB.lol…"
              : mode === "replay"
                ? "Live traffic is unavailable, so the map is showing a clearly labeled replay demonstration."
                : "The last accepted live picture is retained while the source reconnects."}
          </span>
          {!loading && <button type="button" onClick={() => void refresh()}>Try live again</button>}
        </div>
      )}

      <div className="live-tracker-grid">
        <LiveTrackerMap
          aircraft={aircraft}
          selectedId={selected?.id ?? null}
          status={snapshot?.status ?? null}
          mode={mode}
          onSelect={setSelectedId}
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
                <time dateTime={item.observed_at}>{formatAge(item.observed_at)}</time>
              </button>
            ))}
            {!loading && aircraft.length === 0 && <p className="empty-traffic">No aircraft are visible in this regional snapshot.</p>}
          </div>
        </section>
        <AircraftInspector aircraft={selected} mode={mode} />
      </div>

      <footer className="operations-footer live-footer">
        <span>{mode === "replay" ? "Deterministic portfolio replay" : "Best-effort ADS-B positions from ADSB.lol"}</span>
        <span>Map © OpenFreeMap · OpenMapTiles · OpenStreetMap contributors</span>
        <span>UTC / WGS84 · Not for navigation</span>
      </footer>
    </main>
  );
}

function AircraftInspector({ aircraft, mode }: { aircraft: PublicAircraft | null; mode: string }) {
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
          <Fact label="Source quality" value={mode === "replay" ? "Simulated" : aircraft.quality} />
        </dl>
      ) : <p className="empty-traffic">Choose an aircraft to inspect its supplied position and motion facts.</p>}
      <p className="truth-note">Routes, schedules, delays, and airline identity are not inferred when the source does not supply them.</p>
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return <div className="summary-metric"><span>{label}</span><strong>{value}</strong></div>;
}

function replayAircraft(): PublicAircraft[] {
  return PUBLIC_DEMO_FLIGHTS.flatMap((view) => view.latest_position ? [{
    id: view.flight.id,
    callsign: view.flight.callsign,
    aircraft_registration: view.flight.aircraft_registration,
    longitude_degrees: view.latest_position.point.longitude_degrees,
    latitude_degrees: view.latest_position.point.latitude_degrees,
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
