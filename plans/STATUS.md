# Project Status

Last updated: 2026-07-20

## Current state

- Current milestone: M0 — Foundation and feasibility
- Active ticket: None — FT-002 is complete and its post-merge correction awaits human merge
- Branch: `feat/ft-002-canonical-event-model`
- Pull requests: `https://github.com/carlwelchdesign/flight-tracker-ai/pull/3` (merged), `https://github.com/carlwelchdesign/flight-tracker-ai/pull/4` (ready for review)
- Owner: Engineering lead
- Overall status: FT-001 and FT-002 complete; FT-002 correction PR #4 is green; FT-003 is complete on conflicted PR #2
- Next action: Review and merge PR #4, then refresh and verify PR #2 against the updated `main` branch.

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
| M0 | 2 | 3 |
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
- FT-002 implementation is merged through PR #3; correction PR #4 preserves provider revisions while deduplicating identical deliveries and records final evidence.
- The canonical v1 contract separates raw envelopes from normalized facts, uses explicit UTC time/unit/geometry semantics, and enforces operator consistency through composite foreign keys.
- FT-003 is complete on `docs/ft-003-provider-feasibility`; PR #2 remains open for human review and merge.
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is the first live integration target.
- OpenSky may be useful for noncommercial prototyping, but commercial rights and infrastructure restrictions must be resolved before product use.
- FlightAware AeroAPI or an equivalent licensed provider is the likely commercial flight-data path.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
