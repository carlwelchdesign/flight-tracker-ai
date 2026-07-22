"use client";

import { useCallback, useEffect, useState } from "react";
import { parsePublicAirportIntelligence, type PublicAirportIntelligence, type PublicPirep, type PublicTafPeriod } from "@/lib/public-airport-intelligence";

export function AirportIntelligencePanel({ airport, forceOpen = false }: { airport: string; forceOpen?: boolean }) {
  const code = `K${airport}`;
  const [open, setOpen] = useState(false);
  const [snapshot, setSnapshot] = useState<PublicAirportIntelligence | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "ready" | "unavailable">("idle");

  const load = useCallback(async () => {
    setOpen(true); setState("loading");
    try {
      const response = await fetch(`/api/public/airport-intelligence?airport=${code}`, { cache: "no-store" });
      if (!response.ok) throw new Error("unavailable");
      setSnapshot(parsePublicAirportIntelligence(await response.json())); setState("ready");
    } catch { setState("unavailable"); }
  }, [code]);

  useEffect(() => {
    if (!forceOpen || open) return;
    const timer = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(timer);
  }, [forceOpen, load, open]);

  return (
    <section className="airport-intelligence-panel" aria-label={`${code} forecast and nearby pilot reports`}>
      <button type="button" className="airport-intelligence-trigger" aria-expanded={open} onClick={() => open ? setOpen(false) : void load()}>
        <span><strong>{code}</strong> forecast + nearby PIREPs</span><span aria-hidden="true">{open ? "−" : "+"}</span>
      </button>
      {open && <div className="airport-intelligence-body">
        {state === "loading" && <p role="status">Loading NOAA airport intelligence…</p>}
        {state === "unavailable" && <div role="status"><p>Forecast or pilot-report context is unavailable. Aircraft and other weather layers remain usable.</p><button type="button" onClick={() => void load()}>Try again</button></div>}
        {state === "ready" && snapshot && <AirportIntelligence snapshot={snapshot} />}
      </div>}
    </section>
  );
}

function AirportIntelligence({ snapshot }: { snapshot: PublicAirportIntelligence }) {
  return <>
    <header><div><span className={`weather-status weather-status-${snapshot.state === "current" ? "current" : "degraded"}`}><i aria-hidden="true" />{snapshot.state}</span><h3>{snapshot.airport.code} · {snapshot.airport.name}</h3></div><small>Updated {formatTime(snapshot.generated_at)}</small></header>
    <section aria-labelledby="taf-title"><div className="airport-section-heading"><h4 id="taf-title">Terminal forecast</h4><span>{feedLabel(snapshot.taf.state)}</span></div>
      {snapshot.taf.data ? <><p className="airport-source-time">Issued {formatTime(snapshot.taf.data.issue_time)} · valid through {formatTime(snapshot.taf.data.valid_to)}</p><div className="taf-timeline">{snapshot.taf.data.periods.map((period, index) => <TafPeriod key={`${period.valid_from}-${index}`} period={period} />)}</div></> : <p>No accepted TAF is available for this airport.</p>}
    </section>
    <section aria-labelledby="pirep-title"><div className="airport-section-heading"><h4 id="pirep-title">Nearby pilot reports</h4><span>{feedLabel(snapshot.pireps.state)}</span></div>
      {snapshot.pireps.data?.length ? <div className="pirep-list">{snapshot.pireps.data.map((report, index) => <Pirep key={`${report.report_time}-${index}`} report={report} />)}</div> : <p>No located PIREPs were found within 100 NM in the bounded recent window.</p>}
      <p className="airport-coverage-note">{snapshot.coverage_note}</p>
    </section>
    <a className="weather-attribution" href={snapshot.attribution.source_url} target="_blank" rel="noreferrer">{snapshot.attribution.text}</a>
    <small>Advisory portfolio context · not for flight planning or operational use</small>
  </>;
}

function TafPeriod({ period }: { period: PublicTafPeriod }) {
  const cloud = period.clouds.map((layer) => `${layer.coverage}${layer.base_feet_agl ? ` ${layer.base_feet_agl.toLocaleString()} ft` : ""}`).join(", ") || "No cloud layer decoded";
  const wind = period.wind_speed_knots == null ? "Wind unavailable" : `${period.wind_direction_degrees ?? "VRB"}° at ${period.wind_speed_knots} kt${period.wind_gust_knots ? ` gust ${period.wind_gust_knots}` : ""}`;
  return <article><div><strong>{period.change === "BASE" ? "Prevailing" : period.change}</strong>{period.probability_percent != null && <span>{period.probability_percent}%</span>}</div><time>{formatTime(period.valid_from)} – {formatTime(period.valid_to)}</time><p>{wind} · {period.visibility ? `${period.visibility} SM` : "Visibility unavailable"}</p><small>{period.weather ?? cloud}</small></article>;
}

function Pirep({ report }: { report: PublicPirep }) {
  const evidence = [report.turbulence && `Turbulence ${report.turbulence}`, report.icing && `Icing ${report.icing}`, report.clouds && `Cloud ${report.clouds}`, report.wind && `Wind ${report.wind.direction_degrees}°/${report.wind.speed_knots} kt`, report.temperature_celsius != null && `${report.temperature_celsius} °C`, report.weather].filter(Boolean);
  return <article><div><strong>{report.report_type}</strong><span>{report.distance_nautical_miles.toFixed(1)} NM away</span></div><time>{formatTime(report.report_time)}</time><p>{report.altitude_feet == null ? "Altitude not reported" : `${report.altitude_feet.toLocaleString()} ft${report.altitude_context ? ` · ${report.altitude_context}` : ""}`}</p><small>{evidence.length ? evidence.join(" · ") : "No turbulence, icing, cloud, wind, temperature, or weather detail decoded"}</small></article>;
}

function feedLabel(state: "current" | "retained" | "unavailable") { return state === "current" ? "Current fetch" : state === "retained" ? "Last accepted fetch" : "Unavailable"; }
function formatTime(value: string) { return new Intl.DateTimeFormat("en-US", { timeZone: "UTC", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", hour12: false, timeZoneName: "short" }).format(new Date(value)); }
