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
Pull requests: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/3` (merged), `https://github.com/carlwelchdesign/flight-tracker-ai/pull/4` (post-merge correction; merged)
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
- CI gates: PR #3 implementation run `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29778540998`; PR #4 final correction/closeout run `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29782545088`

## FT-003 — Complete provider and API feasibility matrix

Status: Complete

Branch: `docs/ft-003-provider-feasibility`
Final commit: `8c3ed48` (`docs(ft-003): record provider feasibility evidence`)
Pull request: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/2` (ready for review)
Owner: Product and engineering

Compare prototype and commercial sources for flight positions, schedules, weather, hazards, airport conditions, and NOTAMs.

Dependencies: None

Acceptance checklist:

- [x] Each source records licensing, commercial-use rights, coverage, latency, rate limits, history, SLA, and estimated usage cost.
- [x] OpenSky limitations and hosting restrictions are explicitly recorded.
- [x] NOAA endpoints and freshness expectations are confirmed with fixtures.
- [x] FlightAware and at least one credible alternative are evaluated for commercial flight data.
- [x] FAA SWIM/NOTAM access requirements and lead time are documented.
- [x] A provider recommendation or explicit blocked decision is added to `../DECISIONS.md`.

Verification evidence:

- Provider matrix, source register, recommendations, and procurement gates: `../PROVIDER_FEASIBILITY.md`
- Timestamped NOAA METAR, SIGMET, and G-AIRMET observations with revalidated SHA-256 hashes: `../evidence/ft-003/NOAA_API_FIXTURES.md`
- Decision and downstream gate: ADR-007 and OD-002 in `../DECISIONS.md`; R-01, R-11, and R-12 in `../RISKS.md`
- Local gate: changed-document Markdown lint, `git diff --check`, NOAA response-hash verification, and `make verify`
- CI gate: Rust checks, web checks, and API/PostGIS smoke test passed in `https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29777448092`

## FT-004 — Modernize GitHub Actions runtime

Status: Complete

Branch: `chore/ft-004-modernize-ci-actions`
Final commit: `715d7d6` (`ci(ft-004): use Node 24 action runtimes`)
Pull request: [#17](https://github.com/carlwelchdesign/flight-tracker-ai/pull/17)
Owner: Engineering

Remove the deprecated Node 20 action-runtime warnings without changing the application Node.js version or CI behavior.

Dependencies: FT-001

Acceptance checklist:

- [x] Every checkout step uses the current official Node 24 action runtime.
- [x] The web setup action uses the current official Node 24 action runtime while preserving Node.js `20.20.1` for the application build.
- [x] Rust, web, and API/PostGIS jobs still pass.
- [x] The final CI run has no Node 20 action-runtime deprecation annotation.

Verification evidence: official release and action-manifest checks confirmed checkout v7.0.1 and setup-node v7.0.0 use `node24`; CI run [29832129375](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29832129375) passed Rust, web, and API/PostGIS jobs; the three resulting check runs each returned zero annotations; web continued to build and test with application Node.js `20.20.1`.

## FT-005 — Rebaseline the roadmap for a portfolio demonstration

Status: In progress

Branch: `docs/ft-005-portfolio-scope`
Final commit: Pending
Pull request: Pending
Owner: Product and engineering

Make the durable plans reflect the actual goal: a public, non-commercial demonstration for recruiters and hiring managers, with free best-effort data and deterministic replay instead of commercial procurement.

Dependencies: FT-004

Acceptance checklist:

- [x] Product definition names recruiters and hiring managers as the primary audience and prohibits operational-use claims.
- [x] M3 uses an officially eligible free source or replay-only outcome without a commercial provider, paid trial, price, or SLA gate.
- [x] FT-301 and FT-302 preserve a provider-independent Rust boundary, explicit provenance, freshness, rate-limit behavior, and replay fallback.
- [x] M4 targets public portfolio security, reliability, usability, and deployment rather than a real-operator pilot.
- [x] Commercial procurement research remains available only as an optional future production track.
- [x] Status, ticket index, decisions, and risks reflect the same scope.
- [ ] Dedicated branch, intentional commit, pull request, and required checks are recorded.

Verification evidence: `python3 scripts/validate_ft301_evidence.py` and its five-test regression suite pass after the commercial package is moved off the active gate; `git diff --check` and scope-language searches pass; native Rust formatting, strict Clippy, and 74 tests pass; web audit reports zero vulnerabilities, and lint, typecheck, 28 tests, and the production build pass. The local Docker daemon did not respond to the Compose-based wrapper, so Compose configuration and fresh PostGIS verification are delegated to required CI. Final branch, commit, PR, and CI evidence remain pending.
