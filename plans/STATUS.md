# Project Status

Last updated: 2026-07-21

## Current state

- Current milestone: M2 — Live weather and hazard intelligence
- Active ticket: FT-204 — Add alert ranking, lifecycle, and deduplication
- Branch: `feat/ft-204-alert-lifecycle`
- Pull request: Pending
- Owner: Full-stack engineering with dispatcher-workflow review
- Overall status: M0 and M1 are merged; FT-201, FT-202, and FT-203 are merged; FT-204 is in progress
- Next action: Implement explainable alert creation and ranking, stable deduplication, append-only lifecycle actions, and a recoverable dispatcher queue.

## Milestone checklist

- [x] M0 — Foundation and feasibility
- [x] M1 — Simulated operations console
- [ ] M2 — Live weather and hazard intelligence
- [ ] M3 — Commercial flight data and operational workflow
- [ ] M4 — Pilot readiness and operational hardening
- [ ] M5 — Optimization research and controlled recommendations

## Ticket progress

| Milestone | Complete | Total |
| --- | ---: | ---: |
| M0 | 3 | 3 |
| M1 | 4 | 4 |
| M2 | 3 | 4 |
| M3 | 0 | 4 |
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
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is approved as the first live integration target, with explicit source-age and degraded-state handling.
- OpenSky must not be integrated into the automated or commercial product without a written operational/commercial license.
- Cirium Sky Stream and FlightAware Firehose are the commercial flight-data finalists; final selection is blocked on written rights, a common 14-day target-fleet trial, SLA, retention, and price.
- FlightAware AeroAPI must not be used for the dispatcher display under its published self-service license because that license excludes commercial aircraft situational displays.
- FAA SCDS/SWIFT and NMS remain separately access-gated and must not be treated as sole operational sources.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
