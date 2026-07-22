import { describe, expect, it } from "vitest";
import { parsePublicAttentionPicture } from "./public-attention";

type MutablePicture = Record<string, unknown> & {
  aircraft: Array<Record<string, unknown>>;
};

describe("public attention parser", () => {
  it("accepts a bounded deterministic explanation", () => {
    const picture = parsePublicAttentionPicture(validPicture());

    expect(picture.aircraft[0].state).toBe("requires_attention");
    expect(picture.aircraft[0].score?.total).toBe(85);
  });

  it("rejects scored evidence on a non-evaluated aircraft", () => {
    const value = validPicture();
    value.aircraft[0].state = "not_evaluated";

    expect(() => parsePublicAttentionPicture(value)).toThrow(/invented evidence/i);
  });

  it("rejects missing deterministic evidence for an attention state", () => {
    const value = validPicture();
    value.aircraft[0].score = null;

    expect(() => parsePublicAttentionPicture(value)).toThrow(/omitted required/i);
  });
});

function validPicture(): MutablePicture {
  return {
    schema_version: 1,
    scenario_id: "m1-operations-v1",
    scenario_time: "2026-07-21T16:01:00Z",
    source: "portfolio deterministic replay",
    aircraft: [{
      callsign: "FT303",
      state: "requires_attention",
      priority: "critical",
      summary: "A hazard intersects the remaining replay route.",
      observed_facts: [{ label: "Replay route", value: "LAS to SFO · route version 1" }],
      score: {
        hazard_severity_points: 45,
        horizontal_proximity_points: 25,
        altitude_overlap_points: 10,
        time_urgency_points: 5,
        total: 85,
        score_version: 1,
      },
      rule_result: {
        rule_id: "route_hazard_proximity",
        rule_version: 1,
        outcome: "match",
        route_version: 1,
        hazard_revision: 1,
        horizontal_relation: "intersects",
        altitude_relation: "overlap",
      },
      geometric_estimate: {
        closest_approach_nautical_miles: 0,
        proximity_margin_nautical_miles: 25,
        geometry_resolution_nautical_miles: 1,
        disclaimer: "Geometric estimate, not a filed route.",
      },
      source_times: {
        flight_observed_at: "2026-07-21T16:01:00Z",
        hazard_issued_at: "2026-07-21T16:00:00Z",
        evaluated_at: "2026-07-21T16:01:00Z",
      },
    }],
  };
}
