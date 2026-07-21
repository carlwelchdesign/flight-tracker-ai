import { describe, expect, it } from "vitest";
import { parseAlertDetail, parseAlertQueue } from "./alerts-api";

const alert = {
  id: "00000000-0000-0000-0000-000000000001",
  operator_id: "00000000-0000-0000-0000-000000000002",
  event_time: "2026-07-20T16:03:00Z",
  flight_id: "00000000-0000-0000-0000-000000000003",
  hazard_id: "00000000-0000-0000-0000-000000000004",
  alert_type: "route_hazard_proximity",
  severity: "warning",
  lifecycle: "open",
  rule_id: "route_hazard_proximity",
  rule_version: 1,
  series_key: "series-1",
  alert_revision: 1,
  supersedes_alert_id: null,
  attention_score: 80,
  score_version: 1,
  evidence: {
    attention: {
      hazard_severity_points: 45,
      horizontal_proximity_points: 25,
      altitude_overlap_points: 10,
      time_urgency_points: 0,
      total: 80,
      score_version: 1,
    },
    route_hazard: {
      closest_approach_nm: 0,
      proximity_margin_nm: 25,
      route_version: 1,
      hazard_revision: 1,
      horizontal_relation: "intersects",
      altitude_relation: "overlap",
      evaluated_at: "2026-07-20T16:03:00Z",
    },
  },
};

describe("alert API parsing", () => {
  it("keeps versioned score and route-hazard evidence", () => {
    expect(parseAlertQueue({ data: [alert] })[0].evidence.attention.total).toBe(80);
    expect(parseAlertDetail({ ...alert, actions: [] }).score_version).toBe(1);
  });

  it("rejects malformed operational payloads", () => {
    expect(() => parseAlertQueue({ data: [{ ...alert, evidence: null }] })).toThrow(
      /unexpected alert/i,
    );
  });
});
