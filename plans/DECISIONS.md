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

## Open decisions

| ID | Question | Needed by | Resolution evidence |
|---|---|---|---|
| OD-001 | Monorepo package manager and local orchestration approach | FT-001 | Working local setup and contributor ergonomics |
| OD-002 | FlightAware or alternative commercial provider | FT-301 | License, coverage, latency, SLA, and cost matrix |
| OD-003 | SSE versus WebSockets at production scale | M3 | Measured interaction and fan-out requirements |
| OD-004 | Single-tenant pilot versus multi-tenant foundation | FT-303 | Pilot customer constraints and isolation review |
| OD-005 | Whether numerical optimization warrants Python | FT-501 | Benchmark against Rust implementation and library needs |
