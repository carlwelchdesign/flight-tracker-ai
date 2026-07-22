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

### ADR-014 — Atmospheric layers combine fixed NOAA imagery with bounded model vectors

- Date: 2026-07-21
- Decision: Use fixed NOAA nowCOAST WMS products for transparent radar,
  geostationary satellite cloud imagery, and surface-wind barbs. Use a Rust-owned
  allowlisted Open-Meteo GFS/HRRR request only for a small regional grid of
  surface and pressure-level wind vectors needed by the animated field.
- Reason: NOAA's OGC imagery supplies current, visually rich national layers
  without raster processing in the application. A small typed vector grid adds
  directional upper-air motion while preserving an inspectable backend boundary.
- Constraint: Do not expose an arbitrary WMS, coordinate, variable, or pressure
  proxy. Keep all levels and regions server-owned, cache only briefly in memory,
  show product/model time and attribution, and fail independently from aircraft
  and aviation-weather evidence.
- Product boundary: These layers are public portfolio context, not certified
  weather, turbulence/icing analysis, route clearance, or a flight briefing.

### ADR-015 — Explainable public exploration precedes recommendation work

- Date: 2026-07-22
- Decision: After FT-404 closes the public launch gate, deliver four public
  product tickets in order: selected-flight attention explanation, deterministic
  time machine and telemetry, aircraft search and shareable URLs, then airport
  TAF/PIREP intelligence. Continue the existing M5 recommendation and
  human-reviewed drafting work only after this M4.1 sequence. Run FT-403 neutral
  validation on the finished public sequence rather than blocking it beforehand.
- Reason: The public tracker already proves live mapping and atmospheric-layer
  engineering, but its differentiating deterministic decision logic is mostly
  visible only after sign-in. The next portfolio work should make evidence,
  time, and source reasoning understandable before adding another model or
  optimization surface.
- Delivery: FT-413 through FT-416 each use one dedicated feature branch,
  ticket-scoped commits, one pull request targeting `main`, required CI, hosted
  browser evidence, and an updated checklist before completion.
- Constraint: Keep the public experience sanitized and read-only. Do not expose
  protected alerts, tenant data, raw evidence, dispatcher actions, or audit
  history. Do not persist or send ADSB.lol observations to an LLM, and do not
  portray replay, projections, TAFs, or PIREPs as current conditions for a live
  aircraft unless the evidence actually supports that statement.
- M5 boundary: Deterministic code continues to own alert eligibility and
  severity. Optimization remains offline until validated, LLM output remains a
  reviewable draft, and no automatic message or operational action is enabled.

### ADR-016 — Keep authentication out of the public portfolio journey

- Date: 2026-07-22
- Decision: The hosted recruiter experience is the public, read-only tracker.
  It does not invite sign-in, branch into the protected operations console, or
  block public product work on Clerk user/session drills. Direct `/sign-in` and
  `/sign-up` requests return to the tracker.
- Reason: The protected console is an internal engineering surface, not the
  product a recruiter is being asked to evaluate. Its empty production identity
  state repeatedly diverted the project into account setup and error recovery
  without improving the flight-tracker demonstration.
- Security boundary: Protected Rust endpoints continue to reject unauthenticated
  requests, and tenant data, alert actions, notes, evidence URLs, and audit
  history remain non-public. Removing the public auth journey does not open
  those APIs or weaken their tested authorization contracts.
- Delivery consequence: FT-404 closes on the public Vercel/Render/Neon smoke
  evidence. FT-403 becomes a final neutral review of the completed FT-413–416
  public experience instead of a prerequisite that blocks building it.

### ADR-017 — Keep the first offline recommendation experiment in Rust

- Date: 2026-07-22
- Decision: FT-502 ranks at most 12 pre-authored replay route candidates for
  human review using deterministic hard constraints and lexicographic scoring
  in the existing Rust backend.
- Reason: The approved problem is bounded enumeration over the route/hazard
  geometry already owned by Rust, not continuous or mixed-integer optimization.
  The cross-runtime benchmark shows no performance need for a Python service,
  and no Python-only numerical library is required.
- Constraint: The experiment uses project-authored fixtures only, can abstain,
  remains offline, and cannot generate routes, consume live ADS-B/provider data,
  deliver a recommendation, or trigger an operational action.
- Revisit: Re-open only if a later approved experiment demonstrates a required
  Python-only optimization or model library and separately justifies its data,
  deployment, security, observability, and validation boundaries.
- Resolution: OD-005 is resolved in favor of Rust for the bounded FT-502 scope.

## Open decisions

| ID | Question | Needed by | Resolution evidence |
| --- | --- | --- | --- |
| OD-001 | Monorepo package manager and local orchestration approach | FT-001 | Working local setup and contributor ergonomics |
| OD-003 | SSE versus WebSockets at production scale | M3 | Measured interaction and fan-out requirements |
| OD-006 | FAA NMS API or successor path for production NOTAM distribution | Before any post-MVP NOTAM integration | Granted API access, current transition status, schema/coverage validation, service terms, lead time, and permitted operator-facing use |
