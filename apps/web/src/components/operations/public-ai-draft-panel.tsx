"use client";

import { useState } from "react";
import { parsePublicAiDraft, type PublicAiDraft } from "@/lib/public-ai-draft";

type LoadState = "idle" | "loading" | "ready" | "failed";

export function PublicAiDraftPanel() {
  const [state, setState] = useState<LoadState>("idle");
  const [draft, setDraft] = useState<PublicAiDraft | null>(null);

  async function generateDraft() {
    setState("loading");
    try {
      const response = await fetch("/api/public/ai-draft", { cache: "no-store" });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setDraft(parsePublicAiDraft(await response.json()));
      setState("ready");
    } catch {
      setState("failed");
    }
  }

  const generatedByOpenAi = draft?.package.generation.generator === "openai_responses_api";
  return (
    <section className="ops-panel public-ai-draft" aria-labelledby="public-ai-draft-title">
      <div className="public-ai-draft-intro">
        <div>
          <p className="ops-eyebrow">Human-reviewed AI</p>
          <h2 id="public-ai-draft-title">Turn verified evidence into reviewable language</h2>
          <p>OpenAI can word one fixed synthetic recommendation. Rust still owns the facts, validation, fallback, and review boundary.</p>
        </div>
        <button type="button" onClick={() => void generateDraft()} disabled={state === "loading"}>
          {state === "loading" ? "Drafting…" : draft ? "Regenerate cached draft" : "Generate AI draft"}
        </button>
      </div>

      {state === "idle" && <p className="public-ai-placeholder">No live aircraft, weather, tenant, or free-form prompt is sent. The demonstration uses only a versioned project fixture.</p>}
      {state === "failed" && (
        <div className="public-ai-error" role="alert">
          <p>The drafting service is unavailable. The tracker remains fully usable.</p>
          <button type="button" onClick={() => void generateDraft()}>Try again</button>
        </div>
      )}
      {draft && state === "ready" && (
        <div className="public-ai-result" aria-live="polite">
          <div className="public-ai-status-row">
            <span className={generatedByOpenAi ? "ai-status ai-status-live" : "ai-status ai-status-fallback"}>
              {generatedByOpenAi ? `OpenAI · ${draft.package.generation.model ?? "Responses API"}` : "Deterministic fallback"}
            </span>
            <span className="ai-review-status">Awaiting human review</span>
          </div>
          <div className="public-ai-columns">
            <div className="public-ai-facts">
              <h3>Deterministic source facts</h3>
              <dl>
                <Fact label="Candidate" value={draft.package.facts.candidate_id} />
                <Fact label="Closest approach" value={draft.package.facts.closest_approach.display} />
                <Fact label="Added distance" value={`${draft.package.facts.added_distance.display} · ${draft.package.facts.added_distance_percent.display}`} />
                <Fact label="Evidence version" value={`${draft.package.facts.dataset_version} · rule v${draft.package.facts.rule_version}`} />
              </dl>
            </div>
            <article className="public-ai-wording">
              <p className="ops-eyebrow">Generated draft · not approved</p>
              <h3>{draft.package.generated_wording.headline}</h3>
              <p>{draft.package.generated_wording.body}</p>
              <small>{draft.package.generated_wording.caveat}</small>
            </article>
          </div>
          <details className="public-ai-citations">
            <summary>Inspect {draft.package.facts.citations.length} grounded citations</summary>
            <ul>{draft.package.facts.citations.map((citation) => <li key={citation.id}><strong>{citation.label}</strong><span>{citation.source}</span></li>)}</ul>
          </details>
          <p className="public-ai-boundary">{draft.boundary}</p>
        </div>
      )}
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}
