import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PublicAiDraftPanel } from "./public-ai-draft-panel";

afterEach(() => vi.restoreAllMocks());

describe("public AI draft panel", () => {
  it("does not call the model route until the visitor requests the demo", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch");
    render(<PublicAiDraftPanel />);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(screen.getByText(/no live aircraft, weather, tenant, or free-form prompt/i)).toBeInTheDocument();
  });

  it("separates facts, generated wording, and human review state", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify(payload())));
    render(<PublicAiDraftPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Generate AI draft" }));
    expect(await screen.findByText("OpenAI · gpt-5.6-luna")).toBeInTheDocument();
    expect(screen.getByText("Awaiting human review")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Deterministic source facts" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Review candidate" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /approve|send/i })).not.toBeInTheDocument();
  });
});

function payload() {
  return {
    package: {
      facts: {
        case_id: "held-multi-01", candidate_id: "north_arc", dataset_version: "ft502-cases-v1", rule_version: 1,
        closest_approach: { value: 25, unit: "NM", display: "25.0 NM" },
        added_distance: { value: 8, unit: "NM", display: "8.0 NM" },
        added_distance_percent: { value: 2, unit: "%", display: "2.0 %" },
        citations: [{ id: "candidate", label: "Candidate", source: "synthetic fixture", observed_at: "2026-07-22T12:00:00Z" }],
        boundary: "Synthetic only",
      },
      generated_wording: { headline: "Review candidate", body: "Review the evidence.", caveat: "Not operational.", fact_ids: ["candidate"] },
      generation: { generator: "openai_responses_api", model: "gpt-5.6-luna", policy_version: 1, generated_at: "2026-07-22T12:00:00Z", fallback_reason: null },
      review_status: "awaiting_review",
    },
    automatic_send_available: false,
    boundary: "A human must review it; no automatic send is available.",
  };
}
