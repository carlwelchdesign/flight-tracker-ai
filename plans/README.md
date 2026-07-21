# Flight Operations Intelligence — Plan Index

This folder is the durable source of truth for the project. A new contributor or LLM should read this file first, then `STATUS.md`, before changing code or plans.

## Goal

Build an operations-intelligence console inspired by Flight Science: live fleet monitoring, aviation weather and hazard correlation, prioritized dispatcher alerts, and human-reviewed operational messages. The first release is an advisory tool, not a certified flight-planning or autonomous decision system.

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
11. [PROVIDER_FEASIBILITY.md](PROVIDER_FEASIBILITY.md) — provider rights, limits, evidence, and procurement gates
12. [ROADMAP.md](ROADMAP.md) — milestone sequence and gates
13. [tickets/README.md](tickets/README.md) — ticket index and update rules
14. [GIT_WORKFLOW.md](GIT_WORKFLOW.md) — mandatory branch, commit, and PR lifecycle
15. [DECISIONS.md](DECISIONS.md) — decisions and unresolved questions
16. [RISKS.md](RISKS.md) — delivery, safety, licensing, and operational risks

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
