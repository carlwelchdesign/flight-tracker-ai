"use client";

import { useState } from "react";
import Link from "next/link";
import {
  PUBLIC_DEMO_FLIGHTS,
  PUBLIC_DEMO_HAZARDS,
  PUBLIC_DEMO_OBSERVATIONS,
} from "@/lib/public-demo-data";
import { FlightBoard } from "./flight-board";
import { FlightDetail } from "./flight-detail";
import { OperationsMap } from "./operations-map";
import { PortfolioOrientation } from "./portfolio-orientation";

export function PublicFlightTrackerDemo() {
  const [selectedId, setSelectedId] = useState(PUBLIC_DEMO_FLIGHTS[0].flight.id);
  const [selectedHazardId, setSelectedHazardId] = useState<string | null>(null);
  const selected =
    PUBLIC_DEMO_FLIGHTS.find((view) => view.flight.id === selectedId) ?? null;

  return (
    <main className="operations-shell">
      <a className="skip-link" href="#flight-board">Skip to flight board</a>
      <PortfolioOrientation publicDemo />
      <header className="operations-header">
        <div className="product-lockup">
          <span className="product-mark" aria-hidden="true"><i /><i /><i /></span>
          <div>
            <p>Flight Tracker AI</p>
            <span>Public recruiter demo · deterministic replay</span>
          </div>
        </div>

        <div className="operations-summary" aria-label="Demo fleet summary">
          <SummaryMetric label="Tracked" value="3" />
          <SummaryMetric label="Attention" value="2" tone="watch" />
          <SummaryMetric label="Critical" value="0" tone="critical" />
          <SummaryMetric label="Mode" value="Replay" />
        </div>

        <div className="operations-controls">
          <span className="phase phase-active">Interactive read-only demo</span>
          <Link href="/sign-in">Sign in for protected controls</Link>
        </div>
      </header>

      <div className="operations-grid">
        <OperationsMap
          flights={PUBLIC_DEMO_FLIGHTS}
          hazards={PUBLIC_DEMO_HAZARDS}
          observations={PUBLIC_DEMO_OBSERVATIONS}
          sourceHealth={[]}
          weatherState="ready"
          weatherMessage={null}
          weatherAsOf="2026-07-21T16:03:00Z"
          selectedHazardId={selectedHazardId}
          selectedId={selectedId}
          onSelect={setSelectedId}
          onSelectHazard={setSelectedHazardId}
          onRetryWeather={() => undefined}
          liveReferenceTime={null}
        />
        <div id="flight-board">
          <FlightBoard
            flights={PUBLIC_DEMO_FLIGHTS}
            hazards={PUBLIC_DEMO_HAZARDS}
            selectedId={selectedId}
            refreshing={false}
            controlsAvailable={false}
            liveReferenceTime={null}
            onSelect={setSelectedId}
            onStart={() => undefined}
          />
        </div>
        <FlightDetail
          selected={selected}
          flights={PUBLIC_DEMO_FLIGHTS}
          hazards={PUBLIC_DEMO_HAZARDS}
          timeline={[]}
          timelineState="ready"
          liveReferenceTime={null}
        />
      </div>

      <footer className="operations-footer">
        <span>Interactive portfolio flight picture</span>
        <span>Deterministic replay · source-attributed weather</span>
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
