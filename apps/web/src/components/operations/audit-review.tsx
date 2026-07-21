"use client";

import { useEffect, useState } from "react";
import {
  parseAuditEventList,
  parseAuditSignalList,
  type AuditEventList,
  type AuditSignalList,
} from "@/lib/audit-api";
import { formatZulu } from "./operations-model";

type AuditReviewProps = {
  refreshRevision: number;
};

export function AuditReview({ refreshRevision }: AuditReviewProps) {
  const [events, setEvents] = useState<AuditEventList | null>(null);
  const [signals, setSignals] = useState<AuditSignalList | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      try {
        const [eventResponse, signalResponse] = await Promise.all([
          fetch("/api/backend/api/admin/audit-events?limit=50", {
            cache: "no-store",
            signal: controller.signal,
          }),
          fetch("/api/backend/api/admin/audit-alerts", {
            cache: "no-store",
            signal: controller.signal,
          }),
        ]);
        if (!eventResponse.ok || !signalResponse.ok) {
          throw new Error("Audit review is temporarily unavailable");
        }
        setEvents(parseAuditEventList(await eventResponse.json()));
        setSignals(parseAuditSignalList(await signalResponse.json()));
        setError(null);
      } catch (loadError) {
        if (controller.signal.aborted) return;
        setError(loadError instanceof Error ? loadError.message : "Audit review is unavailable");
      }
    }
    void load();
    return () => controller.abort();
  }, [refreshRevision]);

  const exportHref = events
    ? `/api/backend/api/admin/audit-events/export?from=${encodeURIComponent(events.from)}&to=${encodeURIComponent(events.to)}`
    : null;

  return (
    <section className="audit-review" aria-labelledby="audit-review-heading">
      <header>
        <div>
          <p className="section-kicker">Administrator control</p>
          <h2 id="audit-review-heading">Audit review</h2>
          <p>Tenant-scoped authorization and operational actions from the last 24 hours.</p>
        </div>
        <div className="audit-review-actions">
          {signals && (
            <span className={signals.data.length > 0 ? "audit-signal-count has-signals" : "audit-signal-count"}>
              {signals.data.length} monitoring signal{signals.data.length === 1 ? "" : "s"}
            </span>
          )}
          {exportHref && (
            <a href={exportHref} download>
              Export redacted CSV
            </a>
          )}
        </div>
      </header>

      <p className="audit-redaction-note">
        Free-form comments, revocation reasons, idempotency keys, and session identifiers are excluded.
      </p>
      {error && <p className="audit-review-error" role="alert">{error}</p>}
      {!events && !error && <p className="audit-review-loading">Loading audit evidence…</p>}
      {signals && signals.data.length > 0 && (
        <ul className="audit-signals" aria-label="Audit monitoring signals">
          {signals.data.map((signal) => (
            <li key={`${signal.code}-${signal.event_id ?? signal.occurred_at}`}>
              <strong>{signal.severity}</strong>
              <span>{signal.message}</span>
              <span>{signal.actor_id} · {formatZulu(signal.occurred_at)}</span>
            </li>
          ))}
        </ul>
      )}
      {events && events.data.length === 0 && <p>No audit events in this review window.</p>}
      {events && events.data.length > 0 && (
        <ol className="audit-event-list">
          {events.data.map((event) => (
            <li key={`${event.source}-${event.id}`}>
              <span className={`audit-risk audit-risk-${event.risk}`}>{event.risk}</span>
              <strong>{humanize(event.action)}</strong>
              <span>{event.actor_id}</span>
              <span>{event.target_type}{event.target_reference ? ` · ${event.target_reference}` : ""}</span>
              <time dateTime={event.occurred_at}>{formatZulu(event.occurred_at)}</time>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

function humanize(value: string): string {
  return value.replaceAll(/[._]/g, " ").replace(/^./, (letter) => letter.toUpperCase());
}
