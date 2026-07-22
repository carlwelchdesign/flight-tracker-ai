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

### ADR-007 — Gate future operational providers by exact rights

- Date: 2026-07-20
- Decision: Use NOAA Aviation Weather Center data for MVP weather, continue deterministic flight simulation through M1, and evaluate Cirium Sky Stream and FlightAware Firehose as the commercial flight-data finalists.
- Reason: NOAA is technically and legally suitable for advisory weather with explicit freshness handling. Cirium Sky Stream and FlightAware Firehose are marketed for situational or airline operational use, but their decisive rights, target-fleet coverage, SLA, retention, and price are contractual.
- Constraint: Do not integrate OpenSky into an automated or commercial product without a written commercial/operational license. Do not use FlightAware AeroAPI for the dispatcher display under its published self-service license because that license prohibits commercial aircraft situational displays. Do not use FAA SCDS or NMS as a sole operational source without the applicable access agreement and written confirmation of permitted use.
- Gate: If the project is re-scoped for commercial or operational use, select a commercial provider only after the competitive trial and contract checklist in `PROVIDER_FEASIBILITY.md` passes. ADR-010 supersedes this gate for the active portfolio release.

### ADR-008 — Repository commits do not require signing

- Date: 2026-07-20
- Decision: Use ordinary unsigned Git commits for this repository; GPG, SSH signing, and hardware-key confirmation are not delivery gates.
- Reason: Hardware signing repeatedly interrupted otherwise verified ticket delivery without adding a project-specific requirement.
- Constraint: Continue using ticket-scoped branches, intentional Conventional Commits, required CI checks, pull requests, and human-controlled merges.

### ADR-009 — Multi-tenant authorization is app-owned behind a hosted identity adapter

- Date: 2026-07-21
- Decision: Build a multi-tenant foundation now. Use Clerk Organizations as the first web authentication adapter, but keep operator mapping, memberships, roles, session revocation, and authorization policy authoritative in PostgreSQL and Rust.
- Reason: A single-tenant shortcut would require rewriting every operational repository and audit boundary before a second pilot. A provider-neutral internal assertion prevents hosted-provider types and roles from becoming domain authority.
- Constraint: Production requests must use verified hosted sessions. Development uses the same signed-assertion and membership path and is forbidden when `APP_ENV=production`.
- Resolution: OD-004 is resolved in favor of a multi-tenant foundation with a single active operator per verified hosted organization/session.

### ADR-010 — The first release is a non-commercial portfolio demonstration

- Date: 2026-07-21
- Decision: Optimize the current roadmap for a public portfolio demonstration viewed by recruiters and hiring managers, not for airline operations or a commercial SaaS launch.
- Reason: The project exists to demonstrate product and engineering capability. Commercial-provider procurement, an SLA, a 14-day operator trial, and production-operations approval add cost and delay without improving that goal.
- Constraint: Every hosted surface must state that it is a portfolio demonstration and not for operational use. Simulated, live, stale, and unavailable sources must remain distinguishable.
- Data path: Deterministic replay and NOAA weather are the reliable baseline. A free aircraft-position feed is optional and may be integrated only after its official terms are verified for public, hosted, non-commercial display and its attribution and retention rules are implemented.
- Future production: The FT-301 procurement package and ADR-007 research are retained as optional evidence if the product is later re-scoped for commercial or operational use; they no longer gate the portfolio roadmap.

### ADR-011 — ADSB.lol is the optional portfolio live-position source

- Date: 2026-07-21
- Decision: Use ADSB.lol only as an optional, best-effort live aircraft-position layer. Deterministic replay remains the guaranteed demonstration path and the only fallback.
- Reason: ADSB.lol publishes its API and public data under ODbL 1.0, which permits public use and states an attribution path that this hosted non-commercial portfolio can implement. A bounded sample confirmed useful position data as well as missing callsigns, stale positions, and partial service failure.
- Constraint: Fetch only through a bounded Rust regional adapter; poll no faster than every 30 seconds; do not persist, cache, export, or send ADSB.lol data to an LLM; preserve `no-store`; show ODbL attribution whenever the live layer is visible; and revalidate terms and headers before public deployment.
- Product boundary: ADSB.lol supplies identity, position, motion, and source-quality facts only. Routes, schedules, delays, cancellations, and operational statuses remain visibly simulated unless a future source independently proves those facts.
- Resolution: OD-002 is resolved in favor of ADSB.lol under the controls in `ADSBLOL_INTEGRATION.md`; replay-only remains a valid deployment mode.

### ADR-012 — Live trajectories are ephemeral observations plus labeled estimates

- Date: 2026-07-21
- Decision: Build selected-aircraft trajectories from a bounded page-memory trail of accepted ADSB.lol observations and an independently styled five-minute geometric projection from the latest supplied position, true heading, and ground speed.
- Reason: A short observed trail makes motion understandable while a small deterministic projection communicates direction without requiring a route provider, persistence, or AI inference.
- Constraint: Retain at most ten minutes and 25 observations per aircraft; discard them on reload; never write, export, log, analyze, or send them to an LLM. Label the solid line `Observed trail` and the dashed line `Estimated 5-min projection`.
- Product boundary: The projection is presentation-only. It is not a filed route, destination prediction, ETA, conflict forecast, safety recommendation, or new source observation. Missing motion facts produce no projection.

### ADR-013 — Public live coverage uses a curated regional catalog

- Date: 2026-07-21
- Decision: Expand the single SFO picture to seven Rust-owned 50-NM airport regions: SFO, LAX, SEA, DEN, ORD, ATL, and JFK. The browser may select only these identifiers; arbitrary coordinates, radii, and nationwide queries remain unavailable.
- Reason: Recruiters should be able to explore meaningfully different traffic without turning the portfolio into an open ADS-B proxy or implying complete national coverage.
- Rate boundary: Poll each region every 75 seconds, stagger starts evenly across the interval, and retain independent ephemeral status/projection state. This cadence was selected after local live verification found a rolling rate limit at 60 seconds; the 75-second cycle completed with all seven regions current and zero consecutive failures.
- Data boundary: Preserve ADR-011: provider records remain in memory only, are never persisted/exported/sent to an LLM, keep `no-store` and ODbL attribution, and fall back to a clearly simulated regional picture.

## Open decisions

| ID | Question | Needed by | Resolution evidence |
| --- | --- | --- | --- |
| OD-001 | Monorepo package manager and local orchestration approach | FT-001 | Working local setup and contributor ergonomics |
| OD-003 | SSE versus WebSockets at production scale | M3 | Measured interaction and fan-out requirements |
| OD-005 | Whether numerical optimization warrants Python | FT-501 | Benchmark against Rust implementation and library needs |
| OD-006 | FAA NMS API or successor path for production NOTAM distribution | Before any post-MVP NOTAM integration | Granted API access, current transition status, schema/coverage validation, service terms, lead time, and permitted operator-facing use |
