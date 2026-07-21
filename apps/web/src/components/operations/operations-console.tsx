"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { parseBackendHealth } from "@/lib/backend-health";
import type { FleetEvent, FleetLoadResult, FlightPage, FlightView, Hazard, TimelinePage } from "@/lib/fleet-api";
import { hazardFromEvent, parseFleetEvent, parseFlightPage, parseTimelinePage } from "@/lib/fleet-api";
import { FlightBoard } from "./flight-board";
import { FlightDetail } from "./flight-detail";
import { OperationsMap } from "./operations-map";
import { attentionLevel, fleetReferenceTime, fleetTiming, formatZulu } from "./operations-model";
import { OperationsBadges } from "./operations-badges";
import type { ConnectionState, ServiceHealthState } from "./operations-health-model";
import { OperationsStatusRegion } from "./operations-status";

type ReplayPhase = "running" | "paused" | "completed" | "unavailable";

type OperationsConsoleProps = {
  initialFleet: FleetLoadResult;
};

export function OperationsConsole({ initialFleet }: OperationsConsoleProps) {
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
  const [hazards, setHazards] = useState<Hazard[]>([]);
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

  const selected = flights.find((view) => view.flight.id === selectedId) ?? null;
  const referenceTime = fleetReferenceTime(flights);
  const timing = fleetTiming(flights);
  const attentionCounts = useMemo(() => {
    return flights.reduce(
      (counts, view) => {
        counts[attentionLevel(view, hazards, referenceTime).level] += 1;
        return counts;
      },
      { normal: 0, watch: 0, critical: 0 },
    );
  }, [flights, hazards, referenceTime]);

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
      setEventRevision((value) => value + 1);
    }, 80);
  }, [refreshFlights]);

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
            ...current.filter((candidate) => candidate.id !== hazard.id),
            hazard,
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
        const response = await fetch("/api/backend/health", { cache: "no-store" });
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
      setFlights([]);
      setHazards([]);
      setTimeline([]);
      setSelectedId(null);
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

  return (
    <main className="operations-shell">
      <a className="skip-link" href="#flight-board">Skip to flight board</a>
      <header className="operations-header">
        <div className="product-lockup">
          <span className="product-mark" aria-hidden="true"><i /><i /><i /></span>
          <div>
            <p>Flight Tracker AI</p>
            <span>Operations intelligence · advisory</span>
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
                {feedOutage ? "Restore" : "Outage"}
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

      <div className="operations-grid">
        <OperationsMap
          flights={flights}
          hazards={hazards}
          selectedId={selectedId}
          onSelect={selectFlight}
        />
        <div id="flight-board">
          <FlightBoard
            flights={flights}
            hazards={hazards}
            selectedId={selectedId}
            refreshing={refreshing}
            controlsAvailable={replayPhase !== "unavailable"}
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
        />
      </div>

      <footer className="operations-footer">
        <span>Simulation environment</span>
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
