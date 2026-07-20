# M5 — Optimization Research and Controlled Recommendations

Default owner: Data/optimization lead, with aviation-domain and trust review.

## FT-501 — Define optimization feasibility and validation protocol

Status: Not started

Branch: `docs/ft-501-optimization-feasibility`
Final commit: Pending
Pull request: Pending

Specify one narrow recommendation problem, required inputs, baseline, constraints, truth data, and evaluation method before implementing an optimizer.

Dependencies: FT-403

Acceptance checklist:

- [ ] Initial problem is limited to one recommendation class.
- [ ] Required weather, route, aircraft, cost, and outcome data are available lawfully.
- [ ] Safety and operational constraints are represented explicitly.
- [ ] Baseline and held-out evaluation set are documented.
- [ ] Success, failure, and abstention criteria are measurable.
- [ ] Rust-only versus separate Python numerical service is benchmarked and OD-005 is resolved.

Verification evidence: Pending.

## FT-502 — Build an offline recommendation experiment

Status: Not started

Branch: `feat/ft-502-offline-recommendations`
Final commit: Pending
Pull request: Pending

Generate explainable recommendations against historical or simulated scenarios without connecting them to live operations.

Dependencies: FT-501

Acceptance checklist:

- [ ] Experiment is reproducible from versioned data and configuration.
- [ ] Every recommendation records inputs, constraints, model/rule version, and expected effect.
- [ ] System can abstain when evidence or confidence is insufficient.
- [ ] Results are compared to the documented baseline on held-out cases.
- [ ] Domain expert review captures unsafe, impractical, or misleading outputs.
- [ ] No live operational delivery path is enabled.

Verification evidence: Pending.

## FT-503 — Add human-reviewed message drafting

Status: Not started

Branch: `feat/ft-503-human-reviewed-drafts`
Final commit: Pending
Pull request: Pending

Optionally use an LLM to turn approved structured evidence into concise draft language while preserving source traceability and human control.

Dependencies: FT-502

Acceptance checklist:

- [ ] LLM receives only structured, minimized evidence needed for the draft.
- [ ] Draft visibly separates source facts from generated wording.
- [ ] Source timestamps and citations are presented to the reviewer.
- [ ] Reviewer must explicitly approve or edit; no automatic send path exists.
- [ ] Evaluation set measures omissions, fabricated details, unit changes, and unsafe phrasing.
- [ ] Model failure or unavailability degrades to deterministic templates.

Verification evidence: Pending.
