# Flight Operations Intelligence — Plan Index

This folder is the durable source of truth for the project. A new contributor or LLM should read this file first, then `STATUS.md`, before changing code or plans.

## Goal

Build a portfolio demonstration inspired by Flight Science: fleet monitoring, aviation weather and hazard correlation, prioritized alerts, and human-reviewed operational workflows. The primary audience is recruiters and hiring managers evaluating the product thinking and engineering. It is not a certified, commercial, or operational flight-planning system.

## Read order

1. [STATUS.md](STATUS.md) — current milestone, active ticket, and handoff notes
2. [PRODUCT.md](PRODUCT.md) — users, MVP, non-goals, and success measures
3. [ARCHITECTURE.md](ARCHITECTURE.md) — Rust-centered technical design and data flow
4. [CANONICAL_EVENT_MODEL.md](CANONICAL_EVENT_MODEL.md) — provider-independent v1 domain and persistence contract
5. [REPLAY_SCENARIOS.md](REPLAY_SCENARIOS.md) — versioned simulation format, virtual clock, and development controls
6. [FLEET_API.md](FLEET_API.md) — current-state projection, typed reads, SSE reconnect, and metrics
7. [OPERATIONS_CONSOLE.md](OPERATIONS_CONSOLE.md) — dispatcher workflow, interface states, responsiveness, and accessibility contract
8. [OPERATIONS_RUNBOOK.md](OPERATIONS_RUNBOOK.md) — health signals, correlation IDs, outage simulation, and recovery checks
9. [NOAA_INGESTION.md](NOAA_INGESTION.md) — live METAR/SIGMET ingestion, persistence, freshness, and recovery
10. [WEATHER_LAYERS.md](WEATHER_LAYERS.md) — typed weather reads, layer states, evidence access, and performance
11. [ROUTE_HAZARD_RULES.md](ROUTE_HAZARD_RULES.md) — versioned correlation policy, evidence, precision, and golden cases
12. [ALERT_LIFECYCLE.md](ALERT_LIFECYCLE.md) — explainable ranking, dedupe, supersession, human actions, and audit behavior
13. [IDENTITY_TENANT_ISOLATION.md](IDENTITY_TENANT_ISOLATION.md) — identity boundary, app roles, revocation, and fail-closed tenant enforcement
14. [CREDENTIAL_ROTATION_RUNBOOK.md](CREDENTIAL_ROTATION_RUNBOOK.md) — key IDs, planned rotation, emergency revocation, rollback, and drill evidence
15. [AUDIT_REVIEW_RUNBOOK.md](AUDIT_REVIEW_RUNBOOK.md) — administrator review, redacted export, monitoring signals, and incident drill
16. [RETENTION_DELETION_RUNBOOK.md](RETENTION_DELETION_RUNBOOK.md) — two-person policy/run approval, deletion evidence, tombstones, and failure handling
17. [DATA_LIFECYCLE_INCIDENT_POLICY.md](DATA_LIFECYCLE_INCIDENT_POLICY.md) — retention, deletion, backup, restoration, and incident baseline
18. [SECURITY_PRIVACY_TRUST_REVIEW.md](SECURITY_PRIVACY_TRUST_REVIEW.md) — FT-401 threat model, trust controls, and approval gate
19. [PROVIDER_FEASIBILITY.md](PROVIDER_FEASIBILITY.md) — provider rights, limits, evidence, and procurement gates
20. [ADSBLOL_INTEGRATION.md](ADSBLOL_INTEGRATION.md) — selected free position source, ODbL controls, bounded sample, and FT-302 activation gate
21. [ROADMAP.md](ROADMAP.md) — milestone sequence and gates
22. [tickets/README.md](tickets/README.md) — ticket index and update rules
23. [GIT_WORKFLOW.md](GIT_WORKFLOW.md) — mandatory branch, commit, and PR lifecycle
24. [DECISIONS.md](DECISIONS.md) — decisions and unresolved questions
25. [RISKS.md](RISKS.md) — delivery, safety, licensing, and operational risks

## Planning rules

- Check a ticket box only after its acceptance criterion is verified.
- Update both the ticket file and `STATUS.md` when a ticket changes state.
- Deliver each ticket on its own branch with ticket-scoped commits and one PR.
- Add evidence beside completed checks: test command, screenshot, route, commit, or document link.
- Never silently broaden the MVP. Record scope changes in `DECISIONS.md` first.
- Treat every external feed as stale or unavailable until freshness is proven in the UI.
- Preserve human review for dispatcher messages and operational recommendations.
- Do not describe the product as safety-certified unless formal certification work has occurred.

## Status vocabulary

- `Not started` — no implementation work has begun.
- `In progress` — active work exists; `STATUS.md` names the owner and next step.
- `Blocked` — progress requires a recorded dependency or decision.
- `Complete` — every acceptance checkbox is checked and verification evidence is recorded.
- A ticket cannot be `Complete` until its branch, final commit, and PR are recorded and required PR checks pass.

## Current recommendation

Use a TypeScript/Next.js frontend and a Rust backend. Rust is a strong fit for continuously running ingestion, geospatial alert evaluation, predictable resource usage, and typed event processing. Keep optimization behind a later research gate; Python can be introduced as a separate numerical service only if its ecosystem materially accelerates validated trajectory modeling.

Ship the portfolio release with deterministic replay, live NOAA weather, and an optional free best-effort aircraft-position feed whose official terms permit this exact public, non-commercial use. The interface must identify simulated and live sources, expose freshness, and retain replay as the reliable demonstration fallback. Commercial provider procurement, contractual SLAs, and real-operations certification are optional future work and do not block the portfolio release.
