"use client";

import { useCallback, useEffect, useState } from "react";
import type { AlertDetail, AlertQueueItem } from "@/lib/alerts-api";
import { parseAlertDetail, parseAlertQueue } from "@/lib/alerts-api";
import { formatZulu } from "./operations-model";

type AlertQueueProps = {
  operatorId: string | null;
  refreshRevision: number;
};

type LoadState = "idle" | "loading" | "ready" | "error";

export function AlertQueue({ operatorId, refreshRevision }: AlertQueueProps) {
  const [alerts, setAlerts] = useState<AlertQueueItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<AlertDetail | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [actionPending, setActionPending] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState<string | null>(null);

  const loadQueue = useCallback(async () => {
    if (!operatorId) {
      setAlerts([]);
      setLoadState("idle");
      return;
    }
    setLoadState((current) => current === "ready" ? current : "loading");
    try {
      const response = await fetch(
        `/api/backend/api/alerts?operator_id=${encodeURIComponent(operatorId)}`,
        { cache: "no-store" },
      );
      if (!response.ok) throw new Error(`Alert queue returned HTTP ${response.status}`);
      const next = parseAlertQueue(await response.json());
      setAlerts(next);
      setSelectedId((current) =>
        current && next.some((alert) => alert.id === current) ? current : next[0]?.id ?? null,
      );
      setError(null);
      setLoadState("ready");
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : "Alert queue is unavailable");
      setLoadState("error");
    }
  }, [operatorId]);

  useEffect(() => {
    if (!operatorId) return;
    const controller = new AbortController();
    fetch(`/api/backend/api/alerts?operator_id=${encodeURIComponent(operatorId)}`, {
      cache: "no-store",
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Alert queue returned HTTP ${response.status}`);
        return parseAlertQueue(await response.json());
      })
      .then((next) => {
        setAlerts(next);
        setSelectedId((current) =>
          current && next.some((alert) => alert.id === current) ? current : next[0]?.id ?? null,
        );
        setError(null);
        setLoadState("ready");
      })
      .catch((loadError: unknown) => {
        if (loadError instanceof DOMException && loadError.name === "AbortError") return;
        setError(loadError instanceof Error ? loadError.message : "Alert queue is unavailable");
        setLoadState("error");
      });
    return () => controller.abort();
  }, [operatorId, refreshRevision]);

  useEffect(() => {
    if (!operatorId || !selectedId) {
      return;
    }
    const controller = new AbortController();
    fetch(
      `/api/backend/api/alerts/${selectedId}?operator_id=${encodeURIComponent(operatorId)}`,
      { cache: "no-store", signal: controller.signal },
    )
      .then(async (response) => {
        if (!response.ok) throw new Error(`Alert detail returned HTTP ${response.status}`);
        return parseAlertDetail(await response.json());
      })
      .then((next) => {
        setDetail(next);
        setError(null);
      })
      .catch((detailError: unknown) => {
        if (detailError instanceof DOMException && detailError.name === "AbortError") return;
        setError(detailError instanceof Error ? detailError.message : "Alert detail is unavailable");
      });
    return () => controller.abort();
  }, [operatorId, selectedId]);

  async function applyAction(action: "acknowledge" | "comment" | "dismiss" | "resolve") {
    if (!operatorId || !selectedId) return;
    const trimmed = message.trim();
    if (action === "dismiss" && !trimmed) {
      setError("Enter a dismissal reason before dismissing this alert.");
      return;
    }
    if (action === "comment" && !trimmed) {
      setError("Enter a comment before adding it to the audit trail.");
      return;
    }
    setActionPending(true);
    try {
      const response = await fetch(`/api/backend/api/alerts/${selectedId}/actions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          operator_id: operatorId,
          action,
          actor_id: "dispatcher:local",
          idempotency_key: crypto.randomUUID(),
          comment: trimmed || null,
        }),
      });
      const payload: unknown = await response.json();
      if (!response.ok) {
        const apiMessage = readApiError(payload);
        throw new Error(apiMessage ?? `Alert action returned HTTP ${response.status}`);
      }
      setDetail(parseAlertDetail(payload));
      setMessage("");
      setError(null);
      await loadQueue();
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : "Alert action failed");
    } finally {
      setActionPending(false);
    }
  }

  const selectedDetail = detail?.id === selectedId ? detail : null;

  return (
    <section className="ops-panel alert-queue-panel" aria-labelledby="alert-queue-title">
      <div className="ops-panel-heading">
        <div>
          <p className="section-kicker">Decision support</p>
          <h2 id="alert-queue-title">Dispatcher alert queue</h2>
        </div>
        <span className="panel-count">{alerts.length} current</span>
      </div>

      {!operatorId && <QueueMessage title="Waiting for an operator" body="Load a flight picture to scope operational alerts." />}
      {(loadState === "loading" || (loadState === "idle" && operatorId)) && <QueueMessage title="Loading alert evidence" body="Ranking current route and hazard conditions…" />}
      {loadState === "error" && (
        <QueueMessage title="Alert queue unavailable" body={error ?? "The queue could not be loaded."} actionLabel="Retry" onAction={() => void loadQueue()} />
      )}
      {loadState === "ready" && alerts.length === 0 && (
        <QueueMessage title="No current operational alerts" body="New material route–hazard matches will appear here. Clear and indeterminate cases remain suppressed." />
      )}

      {alerts.length > 0 && (
        <div className="alert-workspace">
          <ol className="alert-list" aria-label="Ranked current alerts">
            {alerts.map((alert) => (
              <li key={alert.id}>
                <button
                  type="button"
                  className={selectedId === alert.id ? "alert-row alert-row-selected" : "alert-row"}
                  aria-pressed={selectedId === alert.id}
                  onClick={() => setSelectedId(alert.id)}
                >
                  <span className={`alert-score alert-score-${alert.severity}`}>{alert.attention_score}</span>
                  <span className="alert-row-copy">
                    <strong>{humanize(alert.alert_type)}</strong>
                    <small>{alert.lifecycle} · rev {alert.alert_revision} · {formatZulu(alert.event_time)}</small>
                  </span>
                  <span className={`alert-severity alert-severity-${alert.severity}`}>{alert.severity}</span>
                </button>
              </li>
            ))}
          </ol>

          <div className="alert-inspector" aria-live="polite">
            {!selectedDetail && <QueueMessage title="Loading selected alert" body="Retrieving evidence and audit history…" />}
            {selectedDetail && (
              <>
                <div className="alert-inspector-title">
                  <div>
                    <p className="section-kicker">Why this is ranked</p>
                    <h3>{selectedDetail.attention_score}/100 attention</h3>
                  </div>
                  <span className={`alert-lifecycle alert-lifecycle-${selectedDetail.lifecycle}`}>{selectedDetail.lifecycle}</span>
                </div>
                <dl className="attention-breakdown">
                  <ScorePart label="Hazard severity" value={selectedDetail.evidence.attention.hazard_severity_points} />
                  <ScorePart label="Route proximity" value={selectedDetail.evidence.attention.horizontal_proximity_points} />
                  <ScorePart label="Altitude overlap" value={selectedDetail.evidence.attention.altitude_overlap_points} />
                  <ScorePart label="Time urgency" value={selectedDetail.evidence.attention.time_urgency_points} />
                </dl>
                <p className="alert-evidence-summary">
                  Closest approach {selectedDetail.evidence.route_hazard.closest_approach_nm.toFixed(1)} NM;
                  margin {selectedDetail.evidence.route_hazard.proximity_margin_nm.toFixed(1)} NM.
                  Route v{selectedDetail.evidence.route_hazard.route_version}, hazard r{selectedDetail.evidence.route_hazard.hazard_revision},
                  rule v{selectedDetail.rule_version}, score v{selectedDetail.score_version}.
                </p>

                <label className="alert-note-field">
                  <span>Comment or dismissal reason</span>
                  <textarea value={message} onChange={(event) => setMessage(event.target.value)} rows={2} disabled={actionPending} />
                </label>
                {error && <p className="alert-action-error" role="alert">{error}</p>}
                <div className="alert-actions" aria-label="Alert actions">
                  {selectedDetail.lifecycle === "open" && <button type="button" disabled={actionPending} onClick={() => void applyAction("acknowledge")}>Acknowledge</button>}
                  {selectedDetail.lifecycle !== "dismissed" && selectedDetail.lifecycle !== "resolved" && (
                    <>
                      <button type="button" disabled={actionPending} onClick={() => void applyAction("comment")}>Add comment</button>
                      <button type="button" disabled={actionPending} onClick={() => void applyAction("resolve")}>Resolve</button>
                      <button type="button" className="alert-dismiss" disabled={actionPending} onClick={() => void applyAction("dismiss")}>Dismiss with reason</button>
                    </>
                  )}
                  {(selectedDetail.lifecycle === "dismissed" || selectedDetail.lifecycle === "resolved") && (
                    <button type="button" disabled={actionPending} onClick={() => void applyAction("comment")}>Add follow-up comment</button>
                  )}
                </div>

                <div className="alert-audit">
                  <h4>Append-only audit trail</h4>
                  {selectedDetail.actions.length === 0 ? <p>No dispatcher actions yet.</p> : (
                    <ol>
                      {selectedDetail.actions.map((action) => (
                        <li key={action.id}>
                          <strong>{humanize(action.action)}</strong>
                          <span>{action.actor_id} · {formatZulu(action.occurred_at)}</span>
                          {action.comment && <p>{action.comment}</p>}
                        </li>
                      ))}
                    </ol>
                  )}
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

function ScorePart({ label, value }: { label: string; value: number }) {
  return <div><dt>{label}</dt><dd>+{value}</dd></div>;
}

function QueueMessage({ title, body, actionLabel, onAction }: { title: string; body: string; actionLabel?: string; onAction?: () => void }) {
  return <div className="alert-queue-message"><h3>{title}</h3><p>{body}</p>{actionLabel && onAction && <button type="button" onClick={onAction}>{actionLabel}</button>}</div>;
}

function humanize(value: string): string {
  return value.replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase());
}

function readApiError(value: unknown): string | null {
  if (typeof value !== "object" || value === null || !("error" in value)) return null;
  const error = (value as { error?: unknown }).error;
  if (typeof error !== "object" || error === null || !("message" in error)) return null;
  return typeof (error as { message?: unknown }).message === "string" ? (error as { message: string }).message : null;
}
