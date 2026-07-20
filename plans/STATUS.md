# Project Status

Last updated: 2026-07-20

## Current state

- Current milestone: M0 — Foundation and feasibility
- Active ticket: None
- Overall status: Planning complete; implementation not started
- Next recommended ticket: FT-001 — Scaffold the repository

## Milestone checklist

- [ ] M0 — Foundation and feasibility
- [ ] M1 — Simulated operations console
- [ ] M2 — Live weather and hazard intelligence
- [ ] M3 — Commercial flight data and operational workflow
- [ ] M4 — Pilot readiness and operational hardening
- [ ] M5 — Optimization research and controlled recommendations

## Ticket progress

| Milestone | Complete | Total |
|---|---:|---:|
| M0 | 0 | 3 |
| M1 | 0 | 4 |
| M2 | 0 | 4 |
| M3 | 0 | 4 |
| M4 | 0 | 3 |
| M5 | 0 | 3 |

## Handoff notes

- The workspace contained no project files when this plan was created.
- The workspace is not yet a Git repository. FT-001 must establish `main`, `origin`, a planning baseline commit, and PR tooling before implementation tickets can complete.
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is the first live integration target.
- OpenSky may be useful for noncommercial prototyping, but commercial rights and infrastructure restrictions must be resolved before product use.
- FlightAware AeroAPI or an equivalent licensed provider is the likely commercial flight-data path.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
