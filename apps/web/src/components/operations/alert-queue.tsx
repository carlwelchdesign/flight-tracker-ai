"use client";

import { FormEvent, useCallback, useEffect, useState } from "react";
import {
  buildAlertQueueUrl,
  parseAlertAssignees,
  parseAlertDetail,
  parseAlertQueue,
  type AlertActionKind,
  type AlertAssignee,
  type AlertDetail,
  type AlertLifecycle,
  type AlertQueueFilters,
  type AlertQueueItem,
  type AlertSeverity,
  type DismissalReason,
} from "@/lib/alerts-api";
import { formatZulu } from "./operations-model";

type AlertQueueProps = { canManage: boolean; refreshRevision: number };
type LoadState = "idle" | "loading" | "ready" | "error";
type TimeWindow = "all" | "1h" | "6h" | "24h";

const dismissalReasons: Array<{ value: DismissalReason; label: string }> = [
  { value: "duplicate_alert", label: "Duplicate alert" },
  { value: "stale_source_data", label: "Stale source data" },
  { value: "incorrect_correlation", label: "Incorrect correlation" },
  { value: "not_operationally_relevant", label: "Not operationally relevant" },
  { value: "other", label: "Other" },
];

export function AlertQueue({ canManage, refreshRevision }: AlertQueueProps) {
  const [alerts, setAlerts] = useState<AlertQueueItem[]>([]);
  const [assignees, setAssignees] = useState<AlertAssignee[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<AlertDetail | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [filters, setFilters] = useState<AlertQueueFilters>({});
  const [severity, setSeverity] = useState<AlertSeverity | "all">("all");
  const [status, setStatus] = useState<AlertLifecycle | "all">("all");
  const [flightId, setFlightId] = useState("");
  const [timeWindow, setTimeWindow] = useState<TimeWindow>("all");
  const [assignedTo, setAssignedTo] = useState("all");
  const [selectedAssignee, setSelectedAssignee] = useState("");
  const [dismissalReason, setDismissalReason] = useState<DismissalReason>("duplicate_alert");
  const [message, setMessage] = useState("");
  const [actionPending, setActionPending] = useState<AlertActionKind | null>(null);
  const [feedback, setFeedback] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [assigneeError, setAssigneeError] = useState(false);

  const queueUrl = buildAlertQueueUrl(filters);

  const loadQueue = useCallback(async () => {
    const response = await fetch(queueUrl, { cache: "no-store" });
    if (!response.ok) throw new Error(`Alert queue returned HTTP ${response.status}`);
    const next = parseAlertQueue(await response.json());
    setAlerts(next);
    setSelectedId((current) =>
      current && next.some((alert) => alert.id === current) ? current : next[0]?.id ?? null,
    );
    setLoadState("ready");
    setError(null);
  }, [queueUrl]);

  const loadDetail = useCallback(async (alertId: string) => {
    const response = await fetch(`/api/backend/api/alerts/${alertId}`, { cache: "no-store" });
    if (!response.ok) throw new Error(`Alert detail returned HTTP ${response.status}`);
    const next = parseAlertDetail(await response.json());
    setDetail(next);
    setSelectedAssignee(next.assigned_identity_id ?? "");
    return next;
  }, []);

  useEffect(() => {
    let active = true;
    const queueRequest = fetch(queueUrl, { cache: "no-store" }).then(async (queueResponse) => {
        if (!queueResponse.ok) throw new Error(`Alert queue returned HTTP ${queueResponse.status}`);
        return parseAlertQueue(await queueResponse.json());
      });
    const assigneeRequest = fetch("/api/backend/api/alerts/assignees", { cache: "no-store" }).then(async (assigneeResponse) => {
        if (!assigneeResponse.ok) throw new Error(`Assignees returned HTTP ${assigneeResponse.status}`);
        return parseAlertAssignees(await assigneeResponse.json());
      });
    Promise.allSettled([queueRequest, assigneeRequest])
      .then(([queueResult, assigneeResult]) => {
        if (!active) return;
        if (queueResult.status === "rejected") throw queueResult.reason;
        const nextAlerts = queueResult.value;
        setAlerts(nextAlerts);
        setSelectedId((current) =>
          current && nextAlerts.some((alert) => alert.id === current)
            ? current
            : nextAlerts[0]?.id ?? null,
        );
        setError(null);
        setLoadState("ready");
        if (assigneeResult.status === "fulfilled") {
          setAssignees(assigneeResult.value);
          setAssigneeError(false);
        } else {
          setAssignees([]);
          setAssigneeError(true);
        }
      })
      .catch((loadError: unknown) => {
        if (!active) return;
        setError(loadError instanceof Error ? loadError.message : "Alert queue is unavailable");
        setLoadState("error");
      });
    return () => { active = false; };
  }, [queueUrl, refreshRevision]);

  useEffect(() => {
    if (!selectedId) {
      return;
    }
    let active = true;
    fetch(`/api/backend/api/alerts/${selectedId}`, { cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error(`Alert detail returned HTTP ${response.status}`);
        return parseAlertDetail(await response.json());
      })
      .then((next) => {
        if (!active) return;
        setDetail(next);
        setSelectedAssignee(next.assigned_identity_id ?? "");
      })
      .catch((detailError: unknown) => {
        if (!active) return;
        setError(detailError instanceof Error ? detailError.message : "Alert detail is unavailable");
      });
    return () => { active = false; };
  }, [selectedId]);

  function applyFilters(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const hours = timeWindow === "all" ? null : Number.parseInt(timeWindow, 10);
    setFilters({
      severity: severity === "all" ? undefined : severity,
      status: status === "all" ? undefined : status,
      flight: flightId.trim() || undefined,
      eventFrom: hours ? new Date(Date.now() - hours * 60 * 60 * 1000).toISOString() : undefined,
      assignedTo: assignedTo === "all" ? undefined : assignedTo,
    });
    setLoadState("loading");
    setFeedback("");
  }

  function clearFilters() {
    setSeverity("all");
    setStatus("all");
    setFlightId("");
    setTimeWindow("all");
    setAssignedTo("all");
    setFilters({});
    setLoadState("loading");
  }

  async function applyAction(action: AlertActionKind) {
    if (!selectedId || !detail) return;
    const trimmed = message.trim();
    if (action === "comment" && !trimmed) {
      setError("Enter a comment before adding it to the audit trail.");
      return;
    }
    if (action === "assign" && !selectedAssignee) {
      setError("Choose a dispatcher before assigning this alert.");
      return;
    }
    if (action === "dismiss" && dismissalReason === "other" && !trimmed) {
      setError("Explain the other dismissal reason so alert rules can be tuned.");
      return;
    }
    setActionPending(action);
    setFeedback("");
    try {
      const response = await fetch(`/api/backend/api/alerts/${selectedId}/actions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action,
          idempotency_key: crypto.randomUUID(),
          expected_workflow_version: detail.workflow_version,
          comment: trimmed || null,
          assigned_identity_id: action === "assign" ? selectedAssignee : null,
          dismissal_reason: action === "dismiss" ? dismissalReason : null,
        }),
      });
      const payload: unknown = await response.json();
      if (!response.ok) {
        const apiError = readApiError(payload);
        if (response.status === 409 && apiError?.code === "alert_conflict") {
          await Promise.all([loadDetail(selectedId), loadQueue()]);
          throw new Error("Another dispatcher updated this alert. The latest state is loaded; review it before trying again.");
        }
        throw new Error(apiError?.message ?? `Alert action returned HTTP ${response.status}`);
      }
      const next = parseAlertDetail(payload);
      setDetail(next);
      setSelectedAssignee(next.assigned_identity_id ?? "");
      setMessage("");
      setError(null);
      setFeedback(actionFeedback(action));
      await loadQueue();
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : "Alert action failed");
    } finally {
      setActionPending(null);
    }
  }

  const selectedDetail = detail?.id === selectedId ? detail : null;
  const hasFilters = Object.values(filters).some(Boolean);

  return (
    <section className="ops-panel alert-queue-panel" aria-labelledby="alert-queue-title">
      <div className="ops-panel-heading">
        <div><p className="section-kicker">Decision support</p><h2 id="alert-queue-title">Dispatcher review queue</h2></div>
        <span className="panel-count">{alerts.length} shown</span>
      </div>

      <form className="alert-filter-bar" onSubmit={applyFilters} aria-label="Filter dispatcher alerts">
        <FilterSelect label="Severity" value={severity} onChange={(value) => setSeverity(value as AlertSeverity | "all")} options={["all", "critical", "warning", "advisory", "information"]} />
        <FilterSelect label="Status" value={status} onChange={(value) => setStatus(value as AlertLifecycle | "all")} options={["all", "open", "acknowledged", "dismissed", "resolved"]} />
        <label><span>Flight</span><input value={flightId} onChange={(event) => setFlightId(event.target.value)} placeholder="Callsign or UUID" /></label>
        <FilterSelect label="Event time" value={timeWindow} onChange={(value) => setTimeWindow(value as TimeWindow)} options={["all", "1h", "6h", "24h"]} />
        <label><span>Assigned user</span><select value={assignedTo} onChange={(event) => setAssignedTo(event.target.value)}><option value="all">All</option><option value="unassigned">Unassigned</option>{assignees.map((assignee) => <option key={assignee.identity_id} value={assignee.identity_id}>{assigneeName(assignee)}</option>)}</select></label>
        <div className="alert-filter-actions"><button type="submit">Apply filters</button><button type="button" className="button-secondary" onClick={clearFilters} disabled={!hasFilters}>Clear</button></div>
      </form>

      {(loadState === "loading" || loadState === "idle") && <QueueMessage title="Loading alert evidence" body="Ranking current route and hazard conditions…" />}
      {loadState === "error" && <QueueMessage title="Alert queue unavailable" body={error ?? "The queue could not be loaded."} actionLabel="Retry" onAction={() => void loadQueue()} />}
      {loadState === "ready" && alerts.length === 0 && <QueueMessage title={hasFilters ? "No alerts match these filters" : "No current operational alerts"} body={hasFilters ? "Clear or adjust filters to widen the review queue." : "New material route–hazard matches will appear here. Clear and indeterminate cases remain suppressed."} />}

      {alerts.length > 0 && (
        <div className="alert-workspace">
          <ol className="alert-list" aria-label="Ranked dispatcher alerts">
            {alerts.map((alert) => <li key={alert.id}><button type="button" className={selectedId === alert.id ? "alert-row alert-row-selected" : "alert-row"} aria-pressed={selectedId === alert.id} onClick={() => setSelectedId(alert.id)}><span className={`alert-score alert-score-${alert.severity}`}>{alert.attention_score}</span><span className="alert-row-copy"><strong>{alert.flight_callsign ?? humanize(alert.alert_type)}</strong><small>{humanize(alert.alert_type)} · {alert.lifecycle} · {formatZulu(alert.event_time)}</small><small>{alert.assigned_display_name ?? alert.assigned_subject ?? "Unassigned"}</small></span><span className={`alert-severity alert-severity-${alert.severity}`}>{alert.severity}</span></button></li>)}
          </ol>

          <div className="alert-inspector" aria-live="polite">
            {!selectedDetail && <QueueMessage title="Loading selected alert" body="Retrieving evidence and audit history…" />}
            {selectedDetail && <>
              <div className="alert-inspector-title"><div><p className="section-kicker">Evidence before action</p><h3>{selectedDetail.attention_score}/100 attention</h3></div><span className={`alert-lifecycle alert-lifecycle-${selectedDetail.lifecycle}`}>{selectedDetail.lifecycle}</span></div>
              <dl className="attention-breakdown"><ScorePart label="Hazard severity" value={selectedDetail.evidence.attention.hazard_severity_points} /><ScorePart label="Route proximity" value={selectedDetail.evidence.attention.horizontal_proximity_points} /><ScorePart label="Altitude overlap" value={selectedDetail.evidence.attention.altitude_overlap_points} /><ScorePart label="Time urgency" value={selectedDetail.evidence.attention.time_urgency_points} /></dl>
              <p className="alert-evidence-summary">Closest approach {selectedDetail.evidence.route_hazard.closest_approach_nm.toFixed(1)} NM; margin {selectedDetail.evidence.route_hazard.proximity_margin_nm.toFixed(1)} NM. Route v{selectedDetail.evidence.route_hazard.route_version}, hazard r{selectedDetail.evidence.route_hazard.hazard_revision}, rule v{selectedDetail.rule_version}, score v{selectedDetail.score_version}.</p>

              <div className="alert-assignment-control"><label><span>Assigned dispatcher</span><select value={selectedAssignee} onChange={(event) => setSelectedAssignee(event.target.value)} disabled={Boolean(actionPending) || !canManage || isTerminal(selectedDetail.lifecycle)}><option value="">Choose dispatcher</option>{assignees.map((assignee) => <option key={assignee.identity_id} value={assignee.identity_id}>{assigneeName(assignee)}</option>)}</select></label><button type="button" disabled={Boolean(actionPending) || !canManage || isTerminal(selectedDetail.lifecycle) || !selectedAssignee || selectedAssignee === selectedDetail.assigned_identity_id} onClick={() => void applyAction("assign")}>{actionPending === "assign" ? "Assigning…" : "Assign"}</button></div>
              {assigneeError && <p className="alert-action-error">Assignments are temporarily unavailable. Other review actions remain available.</p>}
              <label className="alert-note-field"><span>Dispatcher note</span><textarea value={message} onChange={(event) => setMessage(event.target.value)} rows={2} maxLength={2000} disabled={Boolean(actionPending) || !canManage} placeholder="Add context for the audit trail (2,000 characters maximum)" /></label>
              {!isTerminal(selectedDetail.lifecycle) && <label className="alert-dismiss-reason"><span>Dismissal reason</span><select value={dismissalReason} onChange={(event) => setDismissalReason(event.target.value as DismissalReason)} disabled={Boolean(actionPending) || !canManage}>{dismissalReasons.map((reason) => <option key={reason.value} value={reason.value}>{reason.label}</option>)}</select></label>}
              {!canManage && <p className="alert-action-error">Viewer access is read-only. A dispatcher, operator, or administrator can act on alerts.</p>}
              {feedback && <p className="alert-action-success" role="status">{feedback}</p>}
              {error && <p className="alert-action-error" role="alert">{error}</p>}
              <div className="alert-actions" aria-label="Alert actions">
                {selectedDetail.lifecycle === "open" && <ActionButton action="acknowledge" pending={actionPending} disabled={!canManage} onAction={applyAction}>Acknowledge</ActionButton>}
                {!isTerminal(selectedDetail.lifecycle) && <><ActionButton action="comment" pending={actionPending} disabled={!canManage} onAction={applyAction}>Add comment</ActionButton><ActionButton action="resolve" pending={actionPending} disabled={!canManage} onAction={applyAction}>Resolve</ActionButton><ActionButton action="dismiss" pending={actionPending} disabled={!canManage} onAction={applyAction} className="alert-dismiss">Dismiss</ActionButton></>}
                {isTerminal(selectedDetail.lifecycle) && <ActionButton action="comment" pending={actionPending} disabled={!canManage} onAction={applyAction}>Add follow-up comment</ActionButton>}
              </div>

              <div className="alert-audit"><h4>Append-only audit trail</h4>{selectedDetail.actions.length === 0 ? <p>No dispatcher actions yet.</p> : <ol>{selectedDetail.actions.map((action) => <li key={action.id}><strong>{humanize(action.action)}</strong><span>{action.actor_id} · {formatZulu(action.occurred_at)}</span>{action.assigned_identity_id && <p>Assigned to {assigneeLabel(assignees, action.assigned_identity_id)}</p>}{action.dismissal_reason && <p>Reason: {humanize(action.dismissal_reason)}</p>}{action.comment && <p>{action.comment}</p>}</li>)}</ol>}</div>
            </>}
          </div>
        </div>
      )}
    </section>
  );
}

function FilterSelect({ label, value, options, onChange }: { label: string; value: string; options: string[]; onChange: (value: string) => void }) { return <label><span>{label}</span><select value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option} value={option}>{option === "all" ? "All" : humanize(option)}</option>)}</select></label>; }
function ActionButton({ action, pending, disabled, onAction, className, children }: { action: AlertActionKind; pending: AlertActionKind | null; disabled: boolean; onAction: (action: AlertActionKind) => Promise<void>; className?: string; children: React.ReactNode }) { return <button type="button" className={className} disabled={Boolean(pending) || disabled} onClick={() => void onAction(action)}>{pending === action ? `${String(children).replace(/^Add /, "Adding ")}…` : children}</button>; }
function ScorePart({ label, value }: { label: string; value: number }) { return <div><dt>{label}</dt><dd>+{value}</dd></div>; }
function QueueMessage({ title, body, actionLabel, onAction }: { title: string; body: string; actionLabel?: string; onAction?: () => void }) { return <div className="alert-queue-message"><h3>{title}</h3><p>{body}</p>{actionLabel && onAction && <button type="button" onClick={onAction}>{actionLabel}</button>}</div>; }
function humanize(value: string): string { return value.replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase()); }
function isTerminal(value: AlertLifecycle): boolean { return value === "dismissed" || value === "resolved"; }
function assigneeName(assignee: AlertAssignee): string { return assignee.display_name ?? assignee.subject; }
function assigneeLabel(assignees: AlertAssignee[], identityId: string): string { return assignees.find((assignee) => assignee.identity_id === identityId)?.display_name ?? identityId; }
function actionFeedback(action: AlertActionKind): string { return ({ acknowledge: "Alert acknowledged.", assign: "Assignment updated.", dismiss: "Alert dismissed with a structured reason.", comment: "Comment added to the audit trail.", resolve: "Alert resolved." })[action]; }
function readApiError(value: unknown): { code: string; message: string } | null { if (typeof value !== "object" || value === null || !("error" in value)) return null; const error = (value as { error?: unknown }).error; if (typeof error !== "object" || error === null) return null; const candidate = error as { code?: unknown; message?: unknown }; return typeof candidate.code === "string" && typeof candidate.message === "string" ? { code: candidate.code, message: candidate.message } : null; }
