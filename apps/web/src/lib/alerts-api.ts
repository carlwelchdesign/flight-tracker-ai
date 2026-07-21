export type AlertLifecycle = "open" | "acknowledged" | "dismissed" | "resolved";
export type AlertSeverity = "information" | "advisory" | "warning" | "critical";
export type AlertActionKind = "acknowledge" | "assign" | "dismiss" | "comment" | "resolve";
export type DismissalReason =
  | "duplicate_alert"
  | "stale_source_data"
  | "incorrect_correlation"
  | "not_operationally_relevant"
  | "other";

export type AttentionBreakdown = {
  hazard_severity_points: number;
  horizontal_proximity_points: number;
  altitude_overlap_points: number;
  time_urgency_points: number;
  total: number;
  score_version: number;
};

export type AlertQueueItem = {
  id: string;
  operator_id: string;
  event_time: string;
  flight_id: string | null;
  flight_callsign: string | null;
  hazard_id: string | null;
  alert_type: string;
  severity: AlertSeverity;
  lifecycle: AlertLifecycle;
  rule_id: string;
  rule_version: number;
  series_key: string;
  alert_revision: number;
  supersedes_alert_id: string | null;
  attention_score: number;
  score_version: number;
  workflow_version: number;
  assigned_identity_id: string | null;
  assigned_subject: string | null;
  assigned_display_name: string | null;
  evidence: {
    attention: AttentionBreakdown;
    route_hazard: {
      closest_approach_nm: number;
      proximity_margin_nm: number;
      route_version: number;
      hazard_revision: number;
      horizontal_relation: string;
      altitude_relation: string;
      evaluated_at: string;
    };
  };
};

export type AlertAction = {
  id: string;
  action: AlertActionKind;
  actor_id: string;
  occurred_at: string;
  comment: string | null;
  assigned_identity_id: string | null;
  dismissal_reason: DismissalReason | null;
};

export type AlertDetail = AlertQueueItem & { actions: AlertAction[] };

export type AlertAssignee = {
  identity_id: string;
  subject: string;
  display_name: string | null;
  role: "dispatcher" | "operator" | "administrator";
};

export type AlertQueueFilters = {
  severity?: AlertSeverity;
  status?: AlertLifecycle;
  flight?: string;
  eventFrom?: string;
  assignedTo?: string;
};

export function buildAlertQueueUrl(filters: AlertQueueFilters): string {
  const query = new URLSearchParams();
  if (filters.severity) query.set("severity", filters.severity);
  if (filters.status) query.set("status", filters.status);
  if (filters.flight?.trim()) query.set("flight", filters.flight.trim());
  if (filters.eventFrom) query.set("event_from", filters.eventFrom);
  if (filters.assignedTo) query.set("assigned_to", filters.assignedTo);
  query.set("limit", "200");
  return `/api/backend/api/alerts?${query}`;
}

export function parseAlertQueue(value: unknown): AlertQueueItem[] {
  if (!isRecord(value) || !Array.isArray(value.data)) {
    throw new Error("Alert API returned an unexpected queue payload");
  }
  return value.data.map(parseAlert);
}

export function parseAlertDetail(value: unknown): AlertDetail {
  const alert = parseAlert(value);
  if (!isRecord(value) || !Array.isArray(value.actions)) {
    throw new Error("Alert API returned an unexpected detail payload");
  }
  return { ...alert, actions: value.actions.map(parseAction) };
}

export function parseAlertAssignees(value: unknown): AlertAssignee[] {
  if (!isRecord(value) || !Array.isArray(value.data)) {
    throw new Error("Alert API returned an unexpected assignee payload");
  }
  return value.data.map((assignee) => {
    if (
      !isRecord(assignee) ||
      typeof assignee.identity_id !== "string" ||
      typeof assignee.subject !== "string" ||
      (assignee.display_name !== null && typeof assignee.display_name !== "string") ||
      !["dispatcher", "operator", "administrator"].includes(String(assignee.role))
    ) {
      throw new Error("Alert API returned an unexpected assignee");
    }
    return assignee as AlertAssignee;
  });
}

function parseAlert(value: unknown): AlertQueueItem {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    typeof value.operator_id !== "string" ||
    typeof value.event_time !== "string" ||
    (value.flight_id !== null && typeof value.flight_id !== "string") ||
    (value.flight_callsign !== null && typeof value.flight_callsign !== "string") ||
    (value.hazard_id !== null && typeof value.hazard_id !== "string") ||
    typeof value.alert_type !== "string" ||
    !["information", "advisory", "warning", "critical"].includes(String(value.severity)) ||
    !["open", "acknowledged", "dismissed", "resolved"].includes(String(value.lifecycle)) ||
    typeof value.attention_score !== "number" ||
    typeof value.alert_revision !== "number" ||
    typeof value.rule_id !== "string" ||
    typeof value.rule_version !== "number" ||
    typeof value.series_key !== "string" ||
    (value.supersedes_alert_id !== null && typeof value.supersedes_alert_id !== "string") ||
    typeof value.score_version !== "number" ||
    typeof value.workflow_version !== "number" ||
    (value.assigned_identity_id !== null && typeof value.assigned_identity_id !== "string") ||
    (value.assigned_subject !== null && typeof value.assigned_subject !== "string") ||
    (value.assigned_display_name !== null && typeof value.assigned_display_name !== "string") ||
    !isRecord(value.evidence) ||
    !isAttention(value.evidence.attention) ||
    !isRouteHazardEvidence(value.evidence.route_hazard)
  ) {
    throw new Error("Alert API returned an unexpected alert");
  }
  return value as AlertQueueItem;
}

function isAttention(value: unknown): value is AttentionBreakdown {
  return isRecord(value) && [
    "hazard_severity_points",
    "horizontal_proximity_points",
    "altitude_overlap_points",
    "time_urgency_points",
    "total",
    "score_version",
  ].every((key) => typeof value[key] === "number");
}

function isRouteHazardEvidence(value: unknown): boolean {
  return isRecord(value) &&
    ["closest_approach_nm", "proximity_margin_nm", "route_version", "hazard_revision"]
      .every((key) => typeof value[key] === "number") &&
    typeof value.horizontal_relation === "string" &&
    typeof value.altitude_relation === "string" &&
    typeof value.evaluated_at === "string";
}

function parseAction(value: unknown): AlertAction {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !["acknowledge", "assign", "dismiss", "comment", "resolve"].includes(String(value.action)) ||
    typeof value.actor_id !== "string" ||
    typeof value.occurred_at !== "string" ||
    (value.comment !== null && typeof value.comment !== "string") ||
    (value.assigned_identity_id !== null && typeof value.assigned_identity_id !== "string") ||
    (value.dismissal_reason !== null && ![
      "duplicate_alert",
      "stale_source_data",
      "incorrect_correlation",
      "not_operationally_relevant",
      "other",
    ].includes(String(value.dismissal_reason)))
  ) {
    throw new Error("Alert API returned an unexpected audit action");
  }
  return value as AlertAction;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
