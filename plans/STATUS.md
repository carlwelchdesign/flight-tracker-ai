# Project Status

Last updated: 2026-07-21

## Current state

- Current milestone: Scope correction — portfolio demonstration
- Active ticket: FT-005 — Rebaseline the roadmap for a portfolio demonstration
- Branch: `docs/ft-005-portfolio-scope`
- Pull request: Pending
- Owner: Product and engineering
- Overall status: M0, M1, and M2 are complete; the commercial-provider gate is being removed from the active portfolio roadmap
- Next action: Merge FT-005, then rebase draft PR #18 onto the portfolio scope and complete FT-301 on its own branch by selecting an officially eligible free best-effort source or recording replay-only as the decision.

## Milestone checklist

- [ ] M0 — Foundation, feasibility, and portfolio rebaseline
- [x] M1 — Simulated operations console
- [x] M2 — Live weather and hazard intelligence
- [ ] M3 — Portfolio live data and operational workflow
- [ ] M4 — Portfolio launch and demonstration hardening
- [ ] M5 — Optimization research and controlled recommendations

## Ticket progress

| Milestone | Complete | Total |
| --- | ---: | ---: |
| M0 | 4 | 5 |
| M1 | 4 | 4 |
| M2 | 4 | 4 |
| M3 | 2 | 4 |
| M4 | 0 | 4 |
| M5 | 0 | 3 |

## Handoff notes

- GitHub repository: `carlwelchdesign/flight-tracker-ai`.
- `main` and `origin` are established; planning baseline commit: `8feb57d`.
- FT-001 is merged through PR #1 at `c8e0bb4`.
- The foundation includes the Rust health/readiness boundary, Next.js interface, PostgreSQL/PostGIS migration, production container targets, one-command startup, and green CI.
- FT-002 implementation is merged through PR #3; correction PR #4 is also merged and preserves provider revisions while deduplicating identical deliveries.
- The canonical v1 contract separates raw envelopes from normalized facts, uses explicit UTC time/unit/geometry semantics, and enforces operator consistency through composite foreign keys.
- FT-003 is merged through PR #2 at `7edfa2a`; M0 is complete.
- FT-101 is merged through PR #5 at `efc2cf6` with all required checks passing.
- FT-102 is merged through PR #6 at `aed432d` with all required checks passing.
- FT-103 is merged through PR #7 at `18a5a23` with all required checks passing.
- FT-104 is merged through PR #8 at `da1a6ad` with all required checks passing.
- FT-201 is merged through PR #9 at `2ce50e2`; Rust, web, and API/PostGIS checks pass, including NOAA persistence and revision behavior.
- FT-202 implementation commit `568bd63` and CI correction `225937f` are green in PR #10; Rust, web, and API/PostGIS checks pass, including weather reads and raw-source evidence against PostGIS.
- FT-202 is merged through PR #10 at `617b337`; local `main` was synchronized before FT-203 began.
- FT-203 is active on `feat/ft-203-route-hazard-rules`; its pure Rust domain rule will remain independent of Axum, SQLx, provider payloads, and wall-clock time.
- FT-203 implementation commit `28f227f` is green in PR #11; Rust, web, and API/PostGIS checks pass, and the PostGIS 3.5 oracle independently confirmed all eight golden cases in CI run `29809973027`.
- FT-203 is merged through PR #11 at `848af8f`; local `main` was synchronized before FT-204 began.
- FT-204 is active on `feat/ft-204-alert-lifecycle`; deterministic Rust policy will own ranking, dedupe, transitions, and audit evidence while the web app exposes human-controlled actions.
- FT-204 is delivered through PR #12. CI run `29811831163` verifies live replay persistence, route-hazard alert creation, score evidence, API acknowledgement, schema invariants, and the independent PostGIS rule oracle; M2 is complete.
- FT-301 is re-scoped to choose an officially eligible free, best-effort aircraft-position source or record replay-only as the outcome. Commercial rights, price, SLA, operator contacts, and a 14-day trial no longer block the portfolio roadmap.
- FT-303 is delivered through PR #13 at implementation commit `1430ce8`. CI run `29814499315` verifies Rust, web, live authenticated replay, the identity migration, PostGIS cross-tenant route isolation, session revocation, and actor/tenant audit behavior. Hosted identity remains behind a provider-neutral boundary; tenant membership and operational authorization are app-owned and enforced by Rust.
- FT-304 is delivered through PR #14 at implementation and CI contract commit `11bdc0d`. CI run `29816346733` verifies the additive migration, authenticated replay, workflow-version acknowledgement, tenant-safe assignment, all queue filters, structured dismissal, conflict rejection, bounded persistence volume, Rust and web quality gates, and production builds.
- FT-301 commercial preparation package is merged through PR #15 at `c8d8a78`; PR #16 records the corrected procurement handoff. This research is retained for an optional future production track and is not an active portfolio-release gate.
- FT-004 upgrades checkout and setup-node to their official Node 24 action-runtime releases through PR #17 at implementation commit `715d7d6`; CI run `29832129375` passed all three jobs with zero check annotations while preserving application Node.js `20.20.1`.
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is approved as the first live integration target, with explicit source-age and degraded-state handling.
- OpenSky must not be integrated into the automated or commercial product without a written operational/commercial license.
- Cirium Sky Stream and FlightAware Firehose remain optional future commercial candidates; no paid provider is required for the portfolio release.
- FlightAware AeroAPI must not be used for the dispatcher display under its published self-service license because that license excludes commercial aircraft situational displays.
- FAA SCDS/SWIFT and NMS remain separately access-gated and must not be treated as sole operational sources.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
