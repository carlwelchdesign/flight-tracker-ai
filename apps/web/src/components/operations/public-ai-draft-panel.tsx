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
  const optionName = draft ? humanizeIdentifier(draft.package.facts.candidate_id) : null;
  return (
    <section className="ops-panel public-ai-draft" aria-labelledby="public-ai-draft-title">
      <div className="public-ai-draft-intro">
        <div>
          <p className="ops-eyebrow">Route comparison</p>
          <h2 id="public-ai-draft-title">Compare a route option</h2>
          <p>This sample shows the tradeoff between keeping distance from a modeled hazard and adding miles to the route.</p>
        </div>
        <button type="button" onClick={() => void generateDraft()} disabled={state === "loading"}>
          {state === "loading" ? "Loading…" : draft ? "Refresh sample" : "Show sample"}
        </button>
      </div>

      {state === "idle" && <p className="public-ai-placeholder">Uses a sample scenario—not live aircraft or weather.</p>}
      {state === "failed" && (
        <div className="public-ai-error" role="alert">
          <p>The example could not be loaded. The live tracker is unaffected.</p>
          <button type="button" onClick={() => void generateDraft()}>Try again</button>
        </div>
      )}
      {draft && state === "ready" && optionName && (
        <div className="public-ai-result" aria-live="polite">
          <div className="public-ai-status-row">
            <span className="ai-status ai-status-live">Sample data</span>
          </div>
          <div className="public-ai-columns">
            <div className="public-ai-facts">
              <h3>Tradeoffs</h3>
              <dl>
                <Fact label="Option" value={optionName} />
                <Fact label="Hazard clearance" value={draft.package.facts.closest_approach.display} />
                <Fact label="Extra distance" value={`${draft.package.facts.added_distance.display} · ${draft.package.facts.added_distance_percent.display}`} />
              </dl>
            </div>
            <article className="public-ai-wording">
              <p className="ops-eyebrow">What this means</p>
              <h3>{optionName} stays {draft.package.facts.closest_approach.display} from the hazard and adds {draft.package.facts.added_distance.display}</h3>
              <p>Compared with the baseline, this option adds {draft.package.facts.added_distance_percent.display} to the route.</p>
            </article>
          </div>
          <details className="public-ai-citations">
            <summary>How this was calculated</summary>
            <p>
              Metrics: {draft.package.facts.dataset_version} · rule v{draft.package.facts.rule_version}.{" "}
              Summary: {generatedByOpenAi ? `OpenAI ${draft.package.generation.model ?? "Responses API"}` : "rules-based fallback"}.
            </p>
            <ul>{draft.package.facts.citations.map((citation) => (
              <li key={citation.id}>
                <strong>{humanizeProvenanceText(citation.label, draft.package.facts.candidate_id, optionName)}</strong>
                <span>{humanizeProvenanceText(citation.source, draft.package.facts.candidate_id, optionName)}</span>
              </li>
            ))}</ul>
          </details>
          <p className="public-ai-boundary">Sample scenario only. Nothing here changes or sends a route.</p>
        </div>
      )}
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}

function humanizeIdentifier(value: string): string {
  const words = value.replaceAll("_", " ").trim().toLowerCase();
  return words ? `${words[0].toUpperCase()}${words.slice(1)}` : "Unnamed option";
}

function humanizeProvenanceText(text: string, identifier: string, optionName: string): string {
  return text
    .replaceAll(`Pre-authored candidate ${identifier}`, `Route option ${optionName}`)
    .replaceAll(`candidate ${identifier}`, `route option ${optionName}`)
    .replaceAll(identifier, optionName)
    .replaceAll("synthetic replay fixture", "sample scenario")
    .replaceAll("synthetic fixture", "sample scenario")
    .replaceAll("deterministic Rust route-hazard rule", "route comparison rule")
    .replaceAll("deterministic Rust candidate comparison", "route comparison rule")
    .replaceAll("Geometric proxy", "Added distance")
    .replaceAll("Fixed-margin geometry", "Hazard clearance")
    .replaceAll("human-approved input", "sample input")
    .replaceAll("FT-502 offline experiment", "sample analysis");
}
