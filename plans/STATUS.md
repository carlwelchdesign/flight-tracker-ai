# Project Status

Last updated: 2026-07-20

## Current state

- Current milestone: M0 — Foundation and feasibility
- Active ticket: FT-003 — Complete provider and API feasibility matrix
- Branch: `docs/ft-003-provider-feasibility`
- Pull request: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/2` (draft)
- Owner: Product and engineering
- Overall status: FT-001 merged; provider feasibility research in progress
- Next action: Verify licensing, coverage, freshness, access, SLA, and cost from primary provider sources.

## Milestone checklist

- [ ] M0 — Foundation and feasibility
- [ ] M1 — Simulated operations console
- [ ] M2 — Live weather and hazard intelligence
- [ ] M3 — Commercial flight data and operational workflow
- [ ] M4 — Pilot readiness and operational hardening
- [ ] M5 — Optimization research and controlled recommendations

## Ticket progress

| Milestone | Complete | Total |
| --- | ---: | ---: |
| M0 | 1 | 3 |
| M1 | 0 | 4 |
| M2 | 0 | 4 |
| M3 | 0 | 4 |
| M4 | 0 | 4 |
| M5 | 0 | 3 |

## Handoff notes

- GitHub repository: `carlwelchdesign/flight-tracker-ai`.
- `main` and `origin` are established; planning baseline commit: `8feb57d`.
- FT-001 is merged through PR #1 at `c8e0bb4`.
- The foundation includes the Rust health/readiness boundary, Next.js interface, PostgreSQL/PostGIS migration, production container targets, one-command startup, and green CI.
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is the first live integration target.
- OpenSky may be useful for noncommercial prototyping, but commercial rights and infrastructure restrictions must be resolved before product use.
- FlightAware AeroAPI or an equivalent licensed provider is the likely commercial flight-data path.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
