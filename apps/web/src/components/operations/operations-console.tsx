"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import Link from "next/link";
import { parseBackendHealth } from "@/lib/backend-health";
import { parseAuthContext, type AuthContext } from "@/lib/auth-model";
import type { FleetEvent, FleetLoadResult, FlightPage, FlightView, TimelinePage } from "@/lib/fleet-api";
import { parseFleetEvent, parseFlightPage, parseTimelinePage } from "@/lib/fleet-api";
import type { LivePositionLoadResult, LivePositionStatus } from "@/lib/live-positions-api";
import { parseLivePositionStatus } from "@/lib/live-positions-api";
import type {
  AirportObservation,
  Hazard,
  WeatherLoadResult,
  WeatherSourceHealth,
} from "@/lib/weather-api";
import {
  hazardFromEvent,
  observationFromEvent,
  parseWeatherSnapshot,
} from "@/lib/weather-api";
import { FlightBoard } from "./flight-board";
import { FlightDetail } from "./flight-detail";
import { OperationsMap } from "./operations-map";
import {
  attentionLevel,
  fleetReferenceTime,
  fleetTiming,
  formatZulu,
  isLivePosition,
} from "./operations-model";
import { OperationsBadges } from "./operations-badges";
import type { ConnectionState, ServiceHealthState } from "./operations-health-model";
import { OperationsStatusRegion } from "./operations-status";
import { AlertQueue } from "./alert-queue";
import { AuditReview } from "./audit-review";
import { LivePositionSource } from "./live-position-source";

type ReplayPhase = "running" | "paused" | "completed" | "unavailable";

type OperationsConsoleProps = {
  orientation: ReactNode;
  authContext: AuthContext;
  initialFleet: FleetLoadResult;
  initialWeather: WeatherLoadResult;
  initialLivePositions: LivePositionLoadResult;
};

export function OperationsConsole({
  orientation,
  authContext,
  initialFleet,
  initialWeather,
  initialLivePositions,
}: OperationsConsoleProps) {
  const [sessionActive, setSessionActive] = useState(true);
  const [flights, setFlights] = useState<FlightView[]>(
    initialFleet.state === "ready" ? initialFleet.page.data : [],
  );
  const [selectedId, setSelectedId] = useState<string | null>(
    initialFleet.state === "ready" ? initialFleet.page.data[0]?.flight.id ?? null : null,
  );
  const [timeline, setTimeline] = useState<FleetEvent[]>([]);
  const [timelineState, setTimelineState] = useState<"idle" | "loading" | "ready" | "error">(
    initialFleet.state === "ready" && initialFleet.page.data.length > 0 ? "loading" : "idle",
  );
  const [hazards, setHazards] = useState<Hazard[]>(
    initialWeather.state === "ready" ? initialWeather.snapshot.hazards : [],
  );
  const [observations, setObservations] = useState<AirportObservation[]>(
    initialWeather.state === "ready" ? initialWeather.snapshot.observations : [],
  );
  const [sourceHealth, setSourceHealth] = useState<WeatherSourceHealth[]>(
    initialWeather.state === "ready" ? initialWeather.snapshot.sourceHealth : [],
  );
  const [weatherState, setWeatherState] = useState<"ready" | "refreshing" | "unavailable">(
    initialWeather.state,
  );
  const [weatherMessage, setWeatherMessage] = useState<string | null>(
    initialWeather.state === "unavailable" ? initialWeather.message : null,
  );
  const [weatherAsOf, setWeatherAsOf] = useState<string | null>(
    initialWeather.state === "ready" ? initialWeather.snapshot.generatedAt : null,
  );
  const [livePositionStatus, setLivePositionStatus] = useState<LivePositionStatus | null>(
    initialLivePositions.state === "ready" ? initialLivePositions.status : null,
  );
  const [livePositionMessage, setLivePositionMessage] = useState<string | null>(
    initialLivePositions.state === "unavailable" ? initialLivePositions.message : null,
  );
  const [selectedHazardId, setSelectedHazardId] = useState<string | null>(null);
  const [connection, setConnection] = useState<ConnectionState>(
    initialFleet.state === "ready" ? "connecting" : "disconnected",
  );
  const [replayPhase, setReplayPhase] = useState<ReplayPhase>("unavailable");
  const [feedOutage, setFeedOutage] = useState(false);
  const [serviceHealth, setServiceHealth] = useState<ServiceHealthState>({
    state: "checking",
    workers: [],
  });
  const [speed, setSpeed] = useState("8x");
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(
    initialFleet.state === "disconnected" ? initialFleet.message : null,
  );
  const [eventRevision, setEventRevision] = useState(0);
  const refreshTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function verifySession() {
      try {
        const response = await fetch("/api/backend/api/auth/context", { cache: "no-store" });
        if (response.status === 401 || response.status === 403) {
          if (!cancelled) setSessionActive(false);
          return;
        }
        if (!response.ok) return;
        parseAuthContext(await response.json());
        if (!cancelled) setSessionActive(true);
      } catch {
        // A transient network error is surfaced by the existing service-health UI.
      }
    }
    void verifySession();
    const interval = setInterval(() => void verifySession(), 15_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const selected = flights.find((view) => view.flight.id === selectedId) ?? null;
  const referenceTime = fleetReferenceTime(flights);
  const liveReferenceTime = parseTimestamp(livePositionStatus?.observed_at ?? null);
  const timing = fleetTiming(flights);
  const attentionCounts = useMemo(() => {
    return flights.reduce(
      (counts, view) => {
        counts[attentionLevel(view, hazards, referenceTime, liveReferenceTime).level] += 1;
        return counts;
      },
      { normal: 0, watch: 0, critical: 0 },
    );
  }, [flights, hazards, liveReferenceTime, referenceTime]);

  const refreshFlights = useCallback(async () => {
    setRefreshing(true);
    try {
      const response = await fetch("/api/backend/api/flights?page=1&page_size=100", {
        cache: "no-store",
      });
      if (!response.ok) throw new Error(`Fleet refresh returned HTTP ${response.status}`);
      const page: FlightPage = parseFlightPage(await response.json());
      setFlights(page.data);
      setSelectedId((current) =>
        current && page.data.some((view) => view.flight.id === current)
          ? current
          : page.data[0]?.flight.id ?? null,
      );
      setError(null);
    } catch (refreshError) {
      setError(refreshError instanceof Error ? refreshError.message : "Fleet refresh failed");
    } finally {
      setRefreshing(false);
    }
  }, []);

  const refreshWeather = useCallback(async () => {
    setWeatherState((current) => current === "ready" ? "refreshing" : current);
    try {
      const [hazardResponse, observationResponse, healthResponse] = await Promise.all([
        fetch("/api/backend/api/hazards", { cache: "no-store" }),
        fetch("/api/backend/api/airport-observations", { cache: "no-store" }),
        fetch("/api/backend/api/source-health", { cache: "no-store" }),
      ]);
      const failed = [hazardResponse, observationResponse, healthResponse].find(
        (response) => !response.ok,
      );
      if (failed) throw new Error(`Weather refresh returned HTTP ${failed.status}`);
      const snapshot = parseWeatherSnapshot(
        await hazardResponse.json(),
        await observationResponse.json(),
        await healthResponse.json(),
      );
      setHazards((current) => mergeProviderFacts(current, snapshot.hazards));
      setObservations((current) => mergeProviderFacts(current, snapshot.observations));
      setSourceHealth(snapshot.sourceHealth);
      setWeatherAsOf(snapshot.generatedAt);
      setWeatherState("ready");
      setWeatherMessage(null);
    } catch (weatherError) {
      setWeatherState("unavailable");
      setWeatherMessage(
        weatherError instanceof Error ? weatherError.message : "Weather refresh failed",
      );
    }
  }, []);

  const refreshLivePositions = useCallback(async () => {
    try {
      const response = await fetch("/api/backend/api/live-positions/status", {
        cache: "no-store",
      });
      if (!response.ok) {
        throw new Error(`Live position status returned HTTP ${response.status}`);
      }
      setLivePositionStatus(parseLivePositionStatus(await response.json()));
      setLivePositionMessage(null);
    } catch (statusError) {
      setLivePositionMessage(
        statusError instanceof Error ? statusError.message : "Live position status is unavailable",
      );
    }
  }, []);

  const selectFlight = useCallback((flightId: string) => {
    setSelectedId(flightId);
    setTimeline([]);
    setTimelineState("loading");
  }, []);

  const scheduleRefresh = useCallback(() => {
    if (refreshTimer.current) return;
    refreshTimer.current = setTimeout(() => {
      refreshTimer.current = null;
      void refreshFlights();
      void refreshWeather();
      void refreshLivePositions();
      setEventRevision((value) => value + 1);
    }, 80);
  }, [refreshFlights, refreshLivePositions, refreshWeather]);

  useEffect(() => {
    const source = new EventSource("/api/backend/api/events/stream");
    source.onopen = () => {
      setConnection("live");
      setError(null);
    };
    const handleFleetEvent = (message: MessageEvent<string>) => {
      try {
        const event = parseFleetEvent(JSON.parse(message.data));
        const hazard = hazardFromEvent(event);
        if (hazard) {
          setHazards((current) => [
            ...current.filter((candidate) => !sameHazardSeries(candidate, hazard)),
            hazard,
          ]);
        }
        const observation = observationFromEvent(event);
        if (observation) {
          setObservations((current) => [
            ...current.filter((candidate) =>
              candidate.operator_id !== observation.operator_id ||
              candidate.station_code !== observation.station_code
            ),
            observation,
          ]);
        }
        scheduleRefresh();
      } catch {
        setError("A live event could not be interpreted. Current state was preserved.");
      }
    };
    source.addEventListener("fleet_event", handleFleetEvent as EventListener);
    source.onerror = () => {
      setConnection((current) => (current === "disconnected" ? current : "reconnecting"));
    };
    return () => {
      if (refreshTimer.current) clearTimeout(refreshTimer.current);
      source.removeEventListener("fleet_event", handleFleetEvent as EventListener);
      source.close();
    };
  }, [scheduleRefresh]);

  useEffect(() => {
    const interval = setInterval(() => void refreshWeather(), 60_000);
    return () => clearInterval(interval);
  }, [refreshWeather]);

  useEffect(() => {
    const interval = setInterval(() => {
      void refreshLivePositions();
      void refreshFlights();
    }, 30_000);
    return () => clearInterval(interval);
  }, [refreshFlights, refreshLivePositions]);

  useEffect(() => {
    let cancelled = false;
    async function loadReplayStatus() {
      try {
        const response = await fetch("/api/backend/api/dev/replay", { cache: "no-store" });
        if (!response.ok) return;
        const payload: unknown = await response.json();
        if (!cancelled && isReplayStatus(payload)) {
          setReplayPhase(payload.phase);
          setSpeed(payload.speed);
          setFeedOutage(payload.feed_outage);
        }
      } catch {
        // Replay controls are optional outside development.
      }
    }
    void loadReplayStatus();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function loadServiceHealth() {
      try {
        const response = await fetch("/api/backend/api/system/health", { cache: "no-store" });
        if (!response.ok) throw new Error(`Health check returned HTTP ${response.status}`);
        const health = parseBackendHealth(await response.json());
        if (cancelled) return;
        if (health.status === "ok") {
          setServiceHealth({ state: "healthy", workers: health.workers });
        } else {
          const degraded = health.workers
            .filter((worker) => worker.state !== "running")
            .map((worker) => `${worker.name}: ${worker.state}`)
            .join(", ");
          setServiceHealth({
            state: "degraded",
            workers: health.workers,
            message: degraded || "A critical worker is not healthy.",
          });
        }
      } catch (healthError) {
        if (cancelled) return;
        setServiceHealth({
          state: "degraded",
          workers: [],
          message: healthError instanceof Error ? healthError.message : "Health check failed",
        });
      }
    }
    void loadServiceHealth();
    const interval = setInterval(() => void loadServiceHealth(), 10_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    if (!selectedId) {
      return;
    }
    const controller = new AbortController();
    fetch(`/api/backend/api/flights/${selectedId}/timeline?page=1&page_size=100`, {
      cache: "no-store",
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Timeline returned HTTP ${response.status}`);
        return parseTimelinePage(await response.json()) as TimelinePage;
      })
      .then((page) => {
        setTimeline(page.data);
        setTimelineState("ready");
      })
      .catch((timelineError: unknown) => {
        if (timelineError instanceof DOMException && timelineError.name === "AbortError") return;
        setTimelineState("error");
      });
    return () => controller.abort();
  }, [selectedId, eventRevision]);

  const invokeReplay = useCallback(
    async (action: "pause" | "resume" | "reset" | "speed" | "outage", body?: unknown) => {
      const response = await fetch(`/api/backend/api/dev/replay/${action}`, {
        method: "POST",
        headers: body ? { "content-type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
      });
      if (!response.ok) throw new Error(`Replay control returned HTTP ${response.status}`);
      const payload: unknown = await response.json();
      if (isReplayStatus(payload)) {
        setReplayPhase(payload.phase);
        setSpeed(payload.speed);
        setFeedOutage(payload.feed_outage);
      }
    },
    [],
  );

  async function startSimulation() {
    try {
      await invokeReplay("speed", { speed });
      await invokeReplay("resume");
      setError(null);
    } catch (controlError) {
      setError(controlError instanceof Error ? controlError.message : "Simulation could not start");
    }
  }

  async function pauseSimulation() {
    try {
      await invokeReplay("pause");
    } catch (controlError) {
      setError(controlError instanceof Error ? controlError.message : "Simulation could not pause");
    }
  }

  async function resetSimulation() {
    try {
      await invokeReplay("reset");
      const liveFlights = flights.filter(isLivePosition);
      setFlights(liveFlights);
      setHazards((current) => current.filter((hazard) => hazard.source.provider !== "simulation"));
      setObservations((current) => current.filter(
        (observation) => observation.source.provider !== "simulation",
      ));
      setSelectedHazardId(null);
      setTimeline([]);
      setSelectedId(liveFlights[0]?.flight.id ?? null);
      setError(null);
    } catch (controlError) {
      setError(controlError instanceof Error ? controlError.message : "Simulation could not reset");
    }
  }

  async function setSimulationOutage(active: boolean) {
    try {
      await invokeReplay("outage", { active });
      setError(null);
    } catch (controlError) {
      setError(
        controlError instanceof Error ? controlError.message : "Feed outage could not be changed",
      );
    }
  }

  function useReplayView() {
    const replayFlight = flights.find((view) => view.flight.source.provider === "simulation");
    if (replayFlight) {
      selectFlight(replayFlight.flight.id);
      return;
    }
    if (replayPhase !== "unavailable") void startSimulation();
  }

  if (!sessionActive) {
    return (
      <main className="session-state">
        <p className="section-kicker">Session ended</p>
        <h1>Operations data has been cleared from view</h1>
        <p>Your session expired or was revoked. Sign in again to restore authorized access.</p>
        <Link href="/sign-in">Open secure sign in</Link>
      </main>
    );
  }

  return (
    <main className="operations-shell">
      <a className="skip-link" href="#flight-board">Skip to flight board</a>
      {orientation}
      <header className="operations-header">
        <div className="product-lockup">
          <span className="product-mark" aria-hidden="true"><i /><i /><i /></span>
          <div>
            <p>Flight Tracker AI</p>
            <span>{authContext.operator_name} · {authContext.role}</span>
          </div>
        </div>

        <div className="operations-summary" aria-label="Fleet summary">
          <SummaryMetric label="Tracked" value={flights.length.toString()} />
          <SummaryMetric label="Attention" value={(attentionCounts.watch + attentionCounts.critical).toString()} tone="watch" />
          <SummaryMetric label="Critical" value={attentionCounts.critical.toString()} tone="critical" />
          <SummaryMetric label="Last event" value={formatTiming(timing.lastEventTime)} />
          <SummaryMetric label="Last received" value={formatTiming(timing.lastReceivedTime)} />
        </div>

        <div className="operations-controls">
          <OperationsBadges connection={connection} serviceHealth={serviceHealth} />
          {replayPhase !== "unavailable" && (
            <div className="replay-controls" aria-label="Simulation controls">
              <label>
                <span className="sr-only">Replay speed</span>
                <select
                  value={speed}
                  onChange={(event) => {
                    const nextSpeed = event.target.value;
                    setSpeed(nextSpeed);
                    void invokeReplay("speed", { speed: nextSpeed }).catch(() => {
                      setError("Replay speed could not be changed");
                    });
                  }}
                >
                  {['0.25x', '0.5x', '1x', '2x', '4x', '8x'].map((value) => (
                    <option key={value} value={value}>{value}</option>
                  ))}
                </select>
              </label>
              {replayPhase === "running" ? (
                <button type="button" onClick={() => void pauseSimulation()}>Pause</button>
              ) : (
                <button type="button" onClick={() => void startSimulation()}>Run</button>
              )}
              <button
                type="button"
                onClick={() => void setSimulationOutage(!feedOutage)}
                aria-label={feedOutage ? "Restore simulation feed" : "Simulate feed outage"}
              >
                {feedOutage ? "Restore feed" : "Test outage"}
              </button>
              <button type="button" className="icon-control" onClick={() => void resetSimulation()} aria-label="Reset simulation">↺</button>
            </div>
          )}
        </div>
      </header>

      <OperationsStatusRegion
        connection={connection}
        serviceHealth={serviceHealth}
        feedOutage={feedOutage}
        error={error}
        onRetry={() => void refreshFlights()}
        onDismiss={() => setError(null)}
        onRestoreFeed={() => void setSimulationOutage(false)}
      />

      <LivePositionSource
        status={livePositionStatus}
        message={livePositionMessage}
        liveFlightsVisible={flights.some(isLivePosition)}
        replayAvailable={
          replayPhase !== "unavailable" ||
          flights.some((view) => view.flight.source.provider === "simulation")
        }
        onUseReplay={useReplayView}
        onRetry={() => void refreshLivePositions()}
      />

      <div className="operations-grid">
        <OperationsMap
          flights={flights}
          hazards={hazards}
          observations={observations}
          sourceHealth={sourceHealth}
          weatherState={weatherState}
          weatherMessage={weatherMessage}
          weatherAsOf={weatherAsOf}
          selectedHazardId={selectedHazardId}
          selectedId={selectedId}
          onSelect={selectFlight}
          onSelectHazard={setSelectedHazardId}
          onRetryWeather={() => void refreshWeather()}
          liveReferenceTime={liveReferenceTime}
        />
        <div id="flight-board">
          <FlightBoard
            flights={flights}
            hazards={hazards}
            selectedId={selectedId}
            refreshing={refreshing}
            controlsAvailable={replayPhase !== "unavailable"}
            liveReferenceTime={liveReferenceTime}
            onSelect={selectFlight}
            onStart={() => void startSimulation()}
          />
        </div>
        <FlightDetail
          selected={selected}
          flights={flights}
          hazards={hazards}
          timeline={timeline}
          timelineState={timelineState}
          liveReferenceTime={liveReferenceTime}
        />
        <div className="alert-queue-slot">
          <AlertQueue
            canManage={authContext.role !== "viewer"}
            refreshRevision={eventRevision}
          />
        </div>
      </div>

      {authContext.role === "administrator" && <AuditReview refreshRevision={eventRevision} />}

      <footer className="operations-footer">
        <span>Advisory evidence workspace</span>
        <span>Source-attributed · human-reviewed decisions</span>
        <span>UTC / WGS84</span>
      </footer>
    </main>
  );
}

function SummaryMetric({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return (
    <div className={tone ? `summary-metric summary-${tone}` : "summary-metric"}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function isReplayStatus(value: unknown): value is {
  phase: Exclude<ReplayPhase, "unavailable">;
  speed: string;
  feed_outage: boolean;
} {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    ["running", "paused", "completed"].includes(String(candidate.phase)) &&
    typeof candidate.speed === "string" &&
    typeof candidate.feed_outage === "boolean"
  );
}

function formatTiming(value: number | null): string {
  return value === null ? "—" : formatZulu(new Date(value).toISOString());
}

function parseTimestamp(value: string | null): number | null {
  if (!value) return null;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : null;
}

function mergeProviderFacts<T extends { id: string; source: { provider: string } }>(
  current: T[],
  persisted: T[],
): T[] {
  const transient = current.filter((item) => item.source.provider === "simulation");
  const transientIds = new Set(transient.map((item) => item.id));
  return [...transient, ...persisted.filter((item) => !transientIds.has(item.id))];
}

function sameHazardSeries(left: Hazard, right: Hazard): boolean {
  return left.operator_id === right.operator_id &&
    left.external_series_id === right.external_series_id;
}
