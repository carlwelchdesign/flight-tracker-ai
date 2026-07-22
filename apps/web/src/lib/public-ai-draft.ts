export type PublicAiDraft = {
  package: {
    facts: {
      case_id: string;
      candidate_id: string;
      dataset_version: string;
      rule_version: number;
      closest_approach: DisplayFact;
      added_distance: DisplayFact;
      added_distance_percent: DisplayFact;
      citations: DraftCitation[];
      boundary: string;
    };
    generated_wording: {
      headline: string;
      body: string;
      caveat: string;
      fact_ids: string[];
    };
    generation: {
      generator: string;
      model: string | null;
      policy_version: number;
      generated_at: string;
      fallback_reason: string | null;
    };
    review_status: "awaiting_review";
  };
  automatic_send_available: false;
  boundary: string;
};

type DisplayFact = { value: number; unit: string; display: string };
type DraftCitation = { id: string; label: string; source: string; observed_at: string };

export function parsePublicAiDraft(value: unknown): PublicAiDraft {
  if (!isRecord(value) || !isRecord(value.package) || value.automatic_send_available !== false || typeof value.boundary !== "string") {
    throw new Error("Public AI draft returned an unexpected payload");
  }
  const { package: draftPackage } = value;
  if (!isRecord(draftPackage.facts) || !isRecord(draftPackage.generated_wording) || !isRecord(draftPackage.generation)) {
    throw new Error("Public AI draft omitted required sections");
  }
  const facts = draftPackage.facts;
  const wording = draftPackage.generated_wording;
  const generation = draftPackage.generation;
  if (
    !strings(facts, ["case_id", "candidate_id", "dataset_version", "boundary"]) ||
    typeof facts.rule_version !== "number" ||
    !isDisplayFact(facts.closest_approach) ||
    !isDisplayFact(facts.added_distance) ||
    !isDisplayFact(facts.added_distance_percent) ||
    !Array.isArray(facts.citations) ||
    !facts.citations.every(isCitation) ||
    !strings(wording, ["headline", "body", "caveat"]) ||
    !Array.isArray(wording.fact_ids) ||
    !wording.fact_ids.every((item) => typeof item === "string") ||
    !strings(generation, ["generator", "generated_at"]) ||
    typeof generation.policy_version !== "number" ||
    (generation.model !== null && typeof generation.model !== "string") ||
    (generation.fallback_reason !== null && typeof generation.fallback_reason !== "string") ||
    draftPackage.review_status !== "awaiting_review"
  ) {
    throw new Error("Public AI draft returned invalid evidence");
  }
  return value as PublicAiDraft;
}

function isDisplayFact(value: unknown): value is DisplayFact {
  return isRecord(value) && typeof value.value === "number" && strings(value, ["unit", "display"]);
}

function isCitation(value: unknown): value is DraftCitation {
  return isRecord(value) && strings(value, ["id", "label", "source", "observed_at"]);
}

function strings(value: Record<string, unknown>, keys: string[]) {
  return keys.every((key) => typeof value[key] === "string");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
