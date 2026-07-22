# Ticket Index

## Update rules

1. Before starting, set the ticket status to `In progress` and update `../STATUS.md`.
2. Create and record the dedicated ticket branch according to `../GIT_WORKFLOW.md`.
3. Keep acceptance boxes unchecked until verified.
4. Add verification evidence directly beneath the relevant ticket.
5. Record the final commit SHA and PR URL in the ticket.
6. Mark a ticket `Complete` only when all boxes are checked and required PR checks pass.
7. Update milestone counts in `../STATUS.md` after completion.
8. Record scope or architecture changes in `../DECISIONS.md`.

Every ticket section has delivery fields. `Branch`, `Final commit`, and `Pull request` may be `Pending` or `Blocked: <reason>` while work is active, but none may remain pending when status becomes `Complete`.

## Tickets

- [M0 — Foundation, feasibility, and scope](M0-foundation.md): FT-001–FT-005
- [M1 — Simulated operations console](M1-simulated-console.md): FT-101–FT-104
- [M2 — Live weather and hazards](M2-weather-hazards.md): FT-201–FT-204
- [M3 — Portfolio live data and workflow](M3-commercial-workflow.md): FT-301–FT-304
- [M4 — Portfolio launch readiness](M4-pilot-readiness.md): FT-401–FT-404
- [M4.1 — Public decision intelligence and exploration](M4.1-public-decision-intelligence.md): FT-413–FT-416
- [FT-417 — Branded social-share metadata](FT-417-social-share-metadata.md)
- [M5 — Optimization research](M5-optimization.md): FT-501–FT-503
