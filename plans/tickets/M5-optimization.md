# M5 — Optimization Research and Controlled Recommendations

Default owner: Data/optimization lead, with aviation-domain and trust review.

## FT-501 — Define optimization feasibility and validation protocol

Status: Complete

Branch: `docs/ft-501-optimization-feasibility`
Final commit: `63efda7`
Pull request: [#48](https://github.com/carlwelchdesign/flight-tracker-ai/pull/48)

Specify one narrow recommendation problem, required inputs, baseline, constraints, truth data, and evaluation method before implementing an optimizer.

Dependencies: FT-403

Acceptance checklist:

- [x] Initial problem is limited to one recommendation class.
- [x] Required weather, route, aircraft, cost, and outcome data are available lawfully.
- [x] Safety and operational constraints are represented explicitly.
- [x] Baseline and held-out evaluation set are documented.
- [x] Success, failure, and abstention criteria are measurable.
- [x] Rust-only versus separate Python numerical service is benchmarked and OD-005 is resolved.

Verification evidence:

- The approved scope, exclusions, repository-owned inputs, hard constraints,
  baseline, 18-case development set, 12-case held-out set, measurable pass/fail
  thresholds, and abstention contract are recorded in
  [`../OPTIMIZATION_FEASIBILITY.md`](../OPTIMIZATION_FEASIBILITY.md).
- Equivalent dependency-free Rust and Python kernels evaluated 800,000 fixed
  candidate paths with the same `73791440.510` checksum. Two July 22, 2026
  development-machine runs measured optimized Rust at 43.971–52.927 ms and
  Python at 1,605.756–2,771.787 ms, a conservative approximately 30x advantage
  when comparing the slower Rust result with the faster Python result. The
  benchmark is reproducible from
  [`../../apps/api/examples/ft501_candidate_benchmark.rs`](../../apps/api/examples/ft501_candidate_benchmark.rs)
  and
  [`../../scripts/ft501_candidate_benchmark.py`](../../scripts/ft501_candidate_benchmark.py).
- ADR-017 selects the existing Rust backend for the bounded FT-502 experiment
  and resolves OD-005. The decision must be revisited before introducing a
  separate numerical service or expanding beyond offline fixture evaluation.
- GitHub CI passed Rust, web, and API/PostGIS smoke checks; Vercel preview also
  passed. PR #48 merged to `main` as `4ca076a`.

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
