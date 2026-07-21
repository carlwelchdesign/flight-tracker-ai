export type AuditRisk = "routine" | "sensitive" | "high";

export type AuditEvent = {
  id: string;
  source: "authorization" | "alert_action";
  actor_id: string;
  action: string;
  target_type: string;
  target_reference: string | null;
  occurred_at: string;
  details: Record<string, string>;
  risk: AuditRisk;
};

export type AuditSignal = {
  code: "high_risk_action" | "privileged_action_burst";
  severity: "warning" | "critical";
  actor_id: string;
  occurred_at: string;
  event_id: string | null;
  message: string;
};

export type AuditEventList = {
  data: AuditEvent[];
  from: string;
  to: string;
};

export type AuditSignalList = {
  data: AuditSignal[];
  since: string;
  through: string;
};

export function parseAuditEventList(value: unknown): AuditEventList {
  if (!isRecord(value) || !Array.isArray(value.data) || !isDate(value.from) || !isDate(value.to)) {
    throw new Error("Audit API returned an unexpected event list");
  }
  return {
    data: value.data.map(parseAuditEvent),
    from: value.from,
    to: value.to,
  };
}

export function parseAuditSignalList(value: unknown): AuditSignalList {
  if (
    !isRecord(value) ||
    !Array.isArray(value.data) ||
    !isDate(value.since) ||
    !isDate(value.through)
  ) {
    throw new Error("Audit API returned an unexpected signal list");
  }
  return {
    data: value.data.map(parseAuditSignal),
    since: value.since,
    through: value.through,
  };
}

function parseAuditEvent(value: unknown): AuditEvent {
  if (
    !isRecord(value) ||
    typeof value.id !== "string" ||
    !["authorization", "alert_action"].includes(String(value.source)) ||
    typeof value.actor_id !== "string" ||
    typeof value.action !== "string" ||
    typeof value.target_type !== "string" ||
    (value.target_reference !== null && typeof value.target_reference !== "string") ||
    !isDate(value.occurred_at) ||
    !["routine", "sensitive", "high"].includes(String(value.risk)) ||
    !isStringRecord(value.details)
  ) {
    throw new Error("Audit API returned an unexpected event");
  }
  return value as AuditEvent;
}

function parseAuditSignal(value: unknown): AuditSignal {
  if (
    !isRecord(value) ||
    !["high_risk_action", "privileged_action_burst"].includes(String(value.code)) ||
    !["warning", "critical"].includes(String(value.severity)) ||
    typeof value.actor_id !== "string" ||
    !isDate(value.occurred_at) ||
    (value.event_id !== null && typeof value.event_id !== "string") ||
    typeof value.message !== "string"
  ) {
    throw new Error("Audit API returned an unexpected monitoring signal");
  }
  return value as AuditSignal;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return isRecord(value) && Object.values(value).every((entry) => typeof entry === "string");
}

function isDate(value: unknown): value is string {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}
