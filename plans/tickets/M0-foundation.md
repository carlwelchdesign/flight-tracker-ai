# M0 — Foundation and Feasibility

Default owner: Engineering lead, with product ownership for provider selection.

## FT-001 — Scaffold the repository

Status: In progress

Branch: `feat/ft-001-repository-scaffold`
Final commit: Pending
Pull request: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/1` (draft)

Create a local development environment for the Next.js interface, Rust backend, PostgreSQL/PostGIS, and shared commands.

Dependencies: None

Acceptance checklist:

- [ ] Repository structure for web, Rust application, migrations, and plans is documented.
- [x] Git repository uses `main`, has a GitHub `origin`, and contains a baseline planning commit.
- [x] Ticket branch and PR conventions are documented and usable with the configured remote.
- [ ] Rust workspace builds and exposes `/health` and `/readiness`.
- [ ] Next.js app loads and can reach the Rust health endpoint.
- [ ] PostgreSQL/PostGIS starts locally and migrations run from a clean database.
- [ ] One documented command starts the development system.
- [ ] Formatting, linting, type checking, and focused tests run in CI.

Verification evidence:

- Repository: `https://github.com/carlwelchdesign/flight-tracker-ai`
- Baseline: `main` at `8feb57d` (`chore: establish project planning baseline`)
- Delivery: `feat/ft-001-repository-scaffold` and draft PR #1

## FT-002 — Define the canonical aviation event model

Status: Not started

Branch: `feat/ft-002-canonical-event-model`
Final commit: Pending
Pull request: Pending

Define versioned Rust types and database tables for provider envelopes, flights, positions, routes, hazards, alerts, actions, and source health.

Dependencies: FT-001

Acceptance checklist:

- [ ] Entity IDs, timestamps, units, nullability, and source attribution are explicit.
- [ ] Raw provider envelopes are separated from normalized facts.
- [ ] Event time, received time, and processed time are preserved.
- [ ] Geometry coordinate order, altitude units, and time conventions are documented and tested.
- [ ] Schema migration and representative serialization tests pass.
- [ ] Tenant/operator boundary is represented before customer data is introduced.

Verification evidence: Pending.

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
