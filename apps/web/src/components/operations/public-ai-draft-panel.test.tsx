import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PublicAiDraftPanel } from "./public-ai-draft-panel";

afterEach(() => vi.restoreAllMocks());

describe("public AI draft panel", () => {
  it("does not call the model route until the visitor requests the demo", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch");
    render(<PublicAiDraftPanel />);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Compare a route option" })).toBeInTheDocument();
    expect(screen.getByText(/uses a sample scenario—not live aircraft or weather/i)).toBeInTheDocument();
    expect(screen.queryByText("Human-reviewed AI")).not.toBeInTheDocument();
  });

  it("presents a plain-language comparison with technical provenance collapsed", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify(payload())));
    render(<PublicAiDraftPanel />);
    fireEvent.click(screen.getByRole("button", { name: "Show sample" }));
    expect(await screen.findByText("Sample data")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Tradeoffs" })).toBeInTheDocument();
    expect(screen.getByText("North arc")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "North arc stays 25.0 NM from the hazard and adds 8.0 NM" })).toBeInTheDocument();
    expect(screen.getByText("Compared with the baseline, this option adds 2.0 % to the route.")).toBeInTheDocument();
    expect(screen.getByText("How this was calculated")).toBeInTheDocument();
    expect(screen.getByText(/Summary: OpenAI gpt-5.6-luna/)).toBeInTheDocument();
    expect(screen.queryByText("north_arc")).not.toBeInTheDocument();
    expect(screen.getByText("Route option North arc")).toBeInTheDocument();
    expect(screen.getByText("sample scenario")).toBeInTheDocument();
    expect(screen.queryByText("Deterministic source facts")).not.toBeInTheDocument();
    expect(screen.queryByText("Generated draft · not approved")).not.toBeInTheDocument();
    expect(screen.queryByText(/deterministic fixture rule/i)).not.toBeInTheDocument();
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
        citations: [{ id: "candidate", label: "Pre-authored candidate north_arc", source: "synthetic fixture", observed_at: "2026-07-22T12:00:00Z" }],
        boundary: "Synthetic only",
      },
      generated_wording: { headline: "Review candidate", body: "Candidate north_arc shows a closest approach of 25.0 NM.", caveat: "Not operational.", fact_ids: ["candidate"] },
      generation: { generator: "openai_responses_api", model: "gpt-5.6-luna", policy_version: 1, generated_at: "2026-07-22T12:00:00Z", fallback_reason: null },
      review_status: "awaiting_review",
    },
    automatic_send_available: false,
    boundary: "A human must review it; no automatic send is available.",
  };
}
