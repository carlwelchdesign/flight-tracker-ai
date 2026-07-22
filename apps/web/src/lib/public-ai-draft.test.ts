import { describe, expect, it } from "vitest";
import { parsePublicAiDraft } from "./public-ai-draft";

const payload = {
  package: {
    facts: {
      case_id: "held-multi-01",
      candidate_id: "north_arc",
      dataset_version: "ft502-cases-v1",
      rule_version: 1,
      closest_approach: { value: 25, unit: "NM", display: "25.0 NM" },
      added_distance: { value: 8, unit: "NM", display: "8.0 NM" },
      added_distance_percent: { value: 2, unit: "%", display: "2.0 %" },
      citations: [{ id: "candidate", label: "Candidate", source: "synthetic fixture", observed_at: "2026-07-22T12:00:00Z" }],
      boundary: "Synthetic only",
    },
    generated_wording: { headline: "Review candidate", body: "Review the facts.", caveat: "Not operational.", fact_ids: ["candidate"] },
    generation: { generator: "openai_responses_api", model: "gpt-5.6-luna", policy_version: 1, generated_at: "2026-07-22T12:00:00Z", fallback_reason: null },
    review_status: "awaiting_review",
  },
  automatic_send_available: false,
  boundary: "Human review required",
};

describe("public AI draft parser", () => {
  it("accepts the bounded awaiting-review contract", () => {
    expect(parsePublicAiDraft(payload).package.generation.generator).toBe("openai_responses_api");
  });

  it("rejects any response that claims automatic send is available", () => {
    expect(() => parsePublicAiDraft({ ...payload, automatic_send_available: true })).toThrow();
  });
});
