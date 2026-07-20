# M0 — Foundation and Feasibility

Default owner: Engineering lead, with product ownership for provider selection.

## FT-001 — Scaffold the repository

Status: Complete

Branch: `feat/ft-001-repository-scaffold`
Final commit: `2f9de16` (`style(ft-001): normalize file endings`)
Pull request: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/1` (merged)

Create a local development environment for the Next.js interface, Rust backend, PostgreSQL/PostGIS, and shared commands.

Dependencies: None

Acceptance checklist:

- [x] Repository structure for web, Rust application, migrations, and plans is documented.
- [x] Git repository uses `main`, has a GitHub `origin`, and contains a baseline planning commit.
- [x] Ticket branch and PR conventions are documented and usable with the configured remote.
- [x] Rust workspace builds and exposes `/health` and `/readiness`.
- [x] Next.js app loads and can reach the Rust health endpoint.
- [x] PostgreSQL/PostGIS starts locally and migrations run from a clean database.
- [x] One documented command starts the development system.
- [x] Formatting, linting, type checking, and focused tests run in CI.

Verification evidence:

- Repository: `https://github.com/carlwelchdesign/flight-tracker-ai`
- Baseline: `main` at `8feb57d` (`chore: establish project planning baseline`)
- Delivery: `feat/ft-001-repository-scaffold`, implementation through `2f9de16`, and PR #1
- Local gate: `make verify`; clean Compose startup; production API and web image builds; live health, readiness, PostGIS, migration, and browser checks
- CI gate: Rust checks, web checks, and API/PostGIS smoke test passed in `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29775876732`

## FT-002 — Define the canonical aviation event model

Status: Complete

Branch: `feat/ft-002-canonical-event-model`
Final commit: `2f01586` (`fix(ft-002): preserve provider record revisions`)
Pull requests: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/3` (merged), `https://github.com/carlwelchdesign/flight-tracker-ai/pull/4` (post-merge correction; ready for review)
Owner: Engineering lead

Define versioned Rust types and database tables for provider envelopes, flights, positions, routes, hazards, alerts, actions, and source health.

Dependencies: FT-001

Acceptance checklist:

- [x] Entity IDs, timestamps, units, nullability, and source attribution are explicit.
- [x] Raw provider envelopes are separated from normalized facts.
- [x] Event time, received time, and processed time are preserved.
- [x] Geometry coordinate order, altitude units, and time conventions are documented and tested.
- [x] Schema migration and representative serialization tests pass.
- [x] Tenant/operator boundary is represented before customer data is introduced.

Verification evidence:

- Rust contract: `../../apps/api/src/domain/`; persisted contract: `../CANONICAL_EVENT_MODEL.md`
- Additive PostGIS migration: `../../migrations/20260720000200_canonical_event_model.sql`
- Unit/serialization gate: 9 Rust unit tests covering schema version, raw/normalized separation, time ordering, coordinates, heading, and representative JSON round trip
- Real-database gate: `schema_contract` passed against a fresh PostGIS 17/3.5 database, including geometry metadata, cross-operator rejection, provider revisions, and duplicate-delivery rejection
- Repository/runtime gate: `make verify`; isolated clean API/database startup with live `/health` and `/readiness`; production Rust image build
- CI gates: PR #3 implementation run `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29778540998`; PR #4 correction run `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29782406127`

## FT-003 — Complete provider and API feasibility matrix

Status: Not started

Branch: `docs/ft-003-provider-feasibility`
Final commit: Pending
Pull request: Pending

Compare prototype and commercial sources for flight positions, schedules, weather, hazards, airport conditions, and NOTAMs.

Dependencies: None

Acceptance checklist:

- [ ] Each source records licensing, commercial-use rights, coverage, latency, rate limits, history, SLA, and estimated usage cost.
- [ ] OpenSky limitations and hosting restrictions are explicitly recorded.
- [ ] NOAA endpoints and freshness expectations are confirmed with fixtures.
- [ ] FlightAware and at least one credible alternative are evaluated for commercial flight data.
- [ ] FAA SWIM/NOTAM access requirements and lead time are documented.
- [ ] A provider recommendation or explicit blocked decision is added to `../DECISIONS.md`.

Verification evidence: Pending.
