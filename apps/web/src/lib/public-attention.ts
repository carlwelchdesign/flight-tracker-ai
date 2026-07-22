export type PublicAttentionState = "requires_attention" | "not_evaluated";
export type PublicAttentionPriority = "information" | "advisory" | "warning" | "critical";

export type PublicAttentionFact = {
  label: string;
  value: string;
};

export type PublicAttentionScore = {
  hazard_severity_points: number;
  horizontal_proximity_points: number;
  altitude_overlap_points: number;
  time_urgency_points: number;
  total: number;
  score_version: number;
};

export type PublicRuleResult = {
  rule_id: string;
  rule_version: number;
  outcome: "match";
  route_version: number;
  hazard_revision: number;
  horizontal_relation: "intersects" | "within_margin" | "clear" | "behind_route_progress";
  altitude_relation: "overlap" | "disjoint" | "indeterminate";
};

export type PublicGeometricEstimate = {
  closest_approach_nautical_miles: number;
  proximity_margin_nautical_miles: number;
  geometry_resolution_nautical_miles: number;
  disclaimer: string;
};

export type PublicAircraftAttention = {
  callsign: string;
  state: PublicAttentionState;
  priority: PublicAttentionPriority | null;
  summary: string;
  observed_facts: PublicAttentionFact[];
  score: PublicAttentionScore | null;
  rule_result: PublicRuleResult | null;
  geometric_estimate: PublicGeometricEstimate | null;
  source_times: {
    flight_observed_at: string | null;
    hazard_issued_at: string | null;
    evaluated_at: string;
  };
};

export type PublicAttentionPicture = {
  schema_version: 1;
  scenario_id: string;
  scenario_time: string;
  source: string;
  aircraft: PublicAircraftAttention[];
};

const MAX_AIRCRAFT = 20;
const MAX_FACTS = 8;

export function parsePublicAttentionPicture(value: unknown): PublicAttentionPicture {
  if (
    !isRecord(value) ||
    value.schema_version !== 1 ||
    !isBoundedString(value.scenario_id, 64) ||
    !isTimestamp(value.scenario_time) ||
    !isBoundedString(value.source, 80) ||
    !Array.isArray(value.aircraft) ||
    value.aircraft.length > MAX_AIRCRAFT
  ) {
    throw new Error("Public attention returned an unexpected payload");
  }
  return {
    schema_version: 1,
    scenario_id: value.scenario_id,
    scenario_time: value.scenario_time,
    source: value.source,
    aircraft: value.aircraft.map(parseAircraftAttention),
  };
}

function parseAircraftAttention(value: unknown): PublicAircraftAttention {
  if (
    !isRecord(value) ||
    !isBoundedString(value.callsign, 16) ||
    !isOneOf(value.state, ["requires_attention", "not_evaluated"]) ||
    !(value.priority === null || isOneOf(value.priority, ["information", "advisory", "warning", "critical"])) ||
    !isBoundedString(value.summary, 320) ||
    !Array.isArray(value.observed_facts) ||
    value.observed_facts.length > MAX_FACTS ||
    !isOptionalScore(value.score) ||
    !isOptionalRuleResult(value.rule_result) ||
    !isOptionalEstimate(value.geometric_estimate) ||
    !isSourceTimes(value.source_times)
  ) {
    throw new Error("Public attention returned an invalid aircraft explanation");
  }
  const observedFacts = value.observed_facts.map(parseFact);
  if (value.state === "requires_attention" && (!value.score || !value.rule_result || !value.geometric_estimate)) {
    throw new Error("Public attention omitted required deterministic evidence");
  }
  if (value.state === "not_evaluated" && (value.score || value.rule_result || value.geometric_estimate)) {
    throw new Error("Public attention invented evidence for a non-evaluated aircraft");
  }
  return {
    callsign: value.callsign,
    state: value.state,
    priority: value.priority,
    summary: value.summary,
    observed_facts: observedFacts,
    score: value.score,
    rule_result: value.rule_result,
    geometric_estimate: value.geometric_estimate,
    source_times: value.source_times,
  } as PublicAircraftAttention;
}

function parseFact(value: unknown): PublicAttentionFact {
  if (!isRecord(value) || !isBoundedString(value.label, 48) || !isBoundedString(value.value, 180)) {
    throw new Error("Public attention returned an invalid evidence fact");
  }
  return { label: value.label, value: value.value };
}

function isOptionalScore(value: unknown): value is PublicAttentionScore | null {
  return value === null || (
    isRecord(value) &&
    isScore(value.hazard_severity_points) &&
    isScore(value.horizontal_proximity_points) &&
    isScore(value.altitude_overlap_points) &&
    isScore(value.time_urgency_points) &&
    isScore(value.total) &&
    isPositiveInteger(value.score_version)
  );
}

function isOptionalRuleResult(value: unknown): value is PublicRuleResult | null {
  return value === null || (
    isRecord(value) &&
    isBoundedString(value.rule_id, 64) &&
    isPositiveInteger(value.rule_version) &&
    value.outcome === "match" &&
    isPositiveInteger(value.route_version) &&
    isPositiveInteger(value.hazard_revision) &&
    isOneOf(value.horizontal_relation, ["intersects", "within_margin", "clear", "behind_route_progress"]) &&
    isOneOf(value.altitude_relation, ["overlap", "disjoint", "indeterminate"])
  );
}

function isOptionalEstimate(value: unknown): value is PublicGeometricEstimate | null {
  return value === null || (
    isRecord(value) &&
    isNonNegativeFinite(value.closest_approach_nautical_miles) &&
    isNonNegativeFinite(value.proximity_margin_nautical_miles) &&
    isNonNegativeFinite(value.geometry_resolution_nautical_miles) &&
    isBoundedString(value.disclaimer, 240)
  );
}

function isSourceTimes(value: unknown): value is PublicAircraftAttention["source_times"] {
  return isRecord(value) &&
    isOptionalTimestamp(value.flight_observed_at) &&
    isOptionalTimestamp(value.hazard_issued_at) &&
    isTimestamp(value.evaluated_at);
}

function isTimestamp(value: unknown): value is string {
  return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function isOptionalTimestamp(value: unknown): value is string | null {
  return value === null || isTimestamp(value);
}

function isBoundedString(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.trim().length > 0 && value.length <= maximum;
}

function isPositiveInteger(value: unknown): value is number {
  return Number.isInteger(value) && Number(value) > 0;
}

function isScore(value: unknown): value is number {
  return Number.isInteger(value) && Number(value) >= 0 && Number(value) <= 100;
}

function isNonNegativeFinite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isOneOf<T extends string>(value: unknown, values: readonly T[]): value is T {
  return typeof value === "string" && values.includes(value as T);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
