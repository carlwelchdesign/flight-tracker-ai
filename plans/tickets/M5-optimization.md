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

Status: In progress

Branch: `feat/ft-502-offline-recommendations`
Final commit: `49c8c85`
Pull request: [#50](https://github.com/carlwelchdesign/flight-tracker-ai/pull/50)

Generate explainable recommendations against historical or simulated scenarios without connecting them to live operations.

Dependencies: FT-501

Acceptance checklist:

- [x] Experiment is reproducible from versioned data and configuration.
- [x] Every recommendation records inputs, constraints, model/rule version, and expected effect.
- [x] System can abstain when evidence or confidence is insufficient.
- [x] Results are compared to the documented baseline on held-out cases.
- [ ] Domain expert review captures unsafe, impractical, or misleading outputs.
- [x] No live operational delivery path is enabled.

Verification evidence:

- `fixtures/optimization/ft502-cases-v1.json` contains 18 development and 12
  sealed held-out project-authored cases with bounded route/hazard templates.
- `apps/api/src/optimization.rs` is a pure offline policy module with no HTTP,
  database, provider, message, or aircraft-control adapter. Results include
  versioned inputs, every constraint outcome, the hazard-blind baseline,
  expected geometric effect, and the mandatory human-review boundary.
- Focused tests prove sealed disposition matching, hard-constraint cleanliness,
  baseline improvement, mandatory abstention, deterministic byte-identical
  output, and absence of delivery/provider payload fields.
- `apps/api/examples/ft502_offline_experiment.rs` emits the reproducible held-out
  comparison report. The captured report records 6 recommendations, 6 correct
  abstentions, a 66.67 percentage-point hazard-clear improvement over baseline,
  2.259% median added distance, zero hard-constraint violations, and
  deterministic output. Independent domain review remains honestly pending in
  [`../evidence/ft-502/DOMAIN_REVIEW.md`](../evidence/ft-502/DOMAIN_REVIEW.md).
- GitHub CI passed Rust, web, and API/PostGIS smoke checks; Vercel preview also
  passed. PR #50 merged to `main` as `8aca521`.

## FT-503 — Add human-reviewed message drafting

Status: Complete

Branch: `feat/ft-503-human-reviewed-drafts`
Final implementation commit: `f7cb5c6`
Pull request: [#52](https://github.com/carlwelchdesign/flight-tracker-ai/pull/52)

Optionally use an LLM to turn approved structured evidence into concise draft language while preserving source traceability and human control.

Dependencies: FT-502

Sequencing note: On 2026-07-22 the project owner explicitly authorized FT-503
engineering to proceed while FT-502's independent aviation-domain review
remains pending. This does not waive or complete the FT-502 review gate.

Acceptance checklist:

- [x] LLM receives only structured, minimized evidence needed for the draft.
- [x] Draft visibly separates source facts from generated wording.
- [x] Source timestamps and citations are presented to the reviewer.
- [x] Reviewer must explicitly approve or edit; no automatic send path exists.
- [x] Evaluation set measures omissions, fabricated details, unit changes, and unsafe phrasing.
- [x] Model failure or unavailability degrades to deterministic templates.

Verification evidence:

- `apps/api/src/drafting/mod.rs` owns the pure minimization, validation,
  deterministic fallback, and explicit review state. It cannot select or alter
  the FT-502 recommendation and has no persistence, HTTP, message, send, or
  aircraft-control adapter.
- `apps/api/src/drafting/openai.rs` is a bounded optional Responses API adapter.
  It sends only `MinimizedDraftEvidence`, requests strict structured output,
  uses `store: false`, caps response bytes and output tokens, and maps provider
  failures to non-sensitive failure codes.
- `fixtures/drafting/ft503-evals-v1.json` contains seven versioned cases. The
  captured [`../evidence/ft-503/EVALUATION_REPORT.json`](../evidence/ft-503/EVALUATION_REPORT.json)
  records 7/7 expected findings matched across grounded output, omission,
  fabrication, unit changes, unsafe phrasing, unknown references, and output
  bounds.
- Twelve focused drafting tests and all 130 Rust library tests pass. Binary,
  integration, example, strict Clippy, formatting, and diff-hygiene checks also
  pass. The offline smoke proves `awaiting_review` to explicit approval and
  reports `automatic_send_available: false`.
- A live OpenAI Responses API probe returned `rate_limited`; the application
  classified the failure, exposed no credential or provider body, and returned
  the valid deterministic fallback in `awaiting_review`. This validates the
  failure path and is not represented as a successful model-generated draft.
- GitHub Actions run
  [29934635940](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29934635940)
  passed Rust, web, and API/PostGIS checks, and the Vercel preview deployment
  passed. PR #52 contains the ticket-scoped implementation and evidence.
