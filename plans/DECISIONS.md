# Decision Log

## Accepted decisions

### ADR-001 — Advisory operations console is the first product

- Date: 2026-07-20
- Decision: Build monitoring, hazard correlation, and human-reviewed alert workflow before optimization or communications integrations.
- Reason: It delivers demonstrable value while limiting safety, integration, and validation risk.

### ADR-002 — Rust owns the operational backend

- Date: 2026-07-20
- Decision: Use Rust for APIs, ingestion, normalization, replay, and deterministic alert evaluation.
- Reason: Strong typing, predictable concurrency, efficient long-running workers, and good control of domain boundaries.
- Constraint: Start as a modular monolith; no microservice split without measured need.

### ADR-003 — PostgreSQL/PostGIS is the system of record

- Date: 2026-07-20
- Decision: Store operational facts, versioned geometry, alerts, and audit records in PostgreSQL/PostGIS.
- Reason: One transactional store can support relational and spatial requirements through the MVP.

### ADR-004 — Simulation precedes paid flight data

- Date: 2026-07-20
- Decision: Build deterministic replay fixtures before selecting a commercial flight provider.
- Reason: It decouples product validation and automated testing from licensing, cost, and feed availability.

### ADR-005 — AI is explanatory, not authoritative

- Date: 2026-07-20
- Decision: LLMs may summarize source material and draft messages, but deterministic code controls alert eligibility and severity; humans approve external messages and recommendations.

### ADR-006 — One branch and pull request per ticket

- Date: 2026-07-20
- Decision: Every implementation ticket uses a dedicated branch, ticket-scoped commits, and one pull request targeting `main`.
- Reason: Ticket-level isolation preserves context across contributors and models, makes acceptance evidence reviewable, and provides a durable delivery history.
- Constraint: Merging remains human-controlled unless the user explicitly authorizes it.

### ADR-007 — Gate live providers by exact operational rights

- Date: 2026-07-20
- Decision: Use NOAA Aviation Weather Center data for MVP weather, continue deterministic flight simulation through M1, and evaluate Cirium Sky Stream and FlightAware Firehose as the commercial flight-data finalists.
- Reason: NOAA is technically and legally suitable for advisory weather with explicit freshness handling. Cirium Sky Stream and FlightAware Firehose are marketed for situational or airline operational use, but their decisive rights, target-fleet coverage, SLA, retention, and price are contractual.
- Constraint: Do not integrate OpenSky into an automated or commercial product without a written commercial/operational license. Do not use FlightAware AeroAPI for the dispatcher display under its published self-service license because that license prohibits commercial aircraft situational displays. Do not use FAA SCDS or NMS as a sole operational source without the applicable access agreement and written confirmation of permitted use.
- Gate: Select a commercial provider only after the competitive trial and contract checklist in `PROVIDER_FEASIBILITY.md` passes. Until then OD-002 remains blocked by procurement evidence.

## Open decisions

| ID | Question | Needed by | Resolution evidence |
| --- | --- | --- | --- |
| OD-001 | Monorepo package manager and local orchestration approach | FT-001 | Working local setup and contributor ergonomics |
| OD-002 | Cirium Sky Stream or FlightAware Firehose for commercial flight data | FT-301 | Written display/retention/combination rights, 14-day target-fleet scorecard, SLA, and priced proposal using `PROVIDER_FEASIBILITY.md` |
| OD-003 | SSE versus WebSockets at production scale | M3 | Measured interaction and fan-out requirements |
| OD-004 | Single-tenant pilot versus multi-tenant foundation | FT-303 | Pilot customer constraints and isolation review |
| OD-005 | Whether numerical optimization warrants Python | FT-501 | Benchmark against Rust implementation and library needs |
| OD-006 | FAA NMS API or successor path for production NOTAM distribution | Before any post-MVP NOTAM integration | Granted API access, current transition status, schema/coverage validation, service terms, lead time, and permitted operator-facing use |
