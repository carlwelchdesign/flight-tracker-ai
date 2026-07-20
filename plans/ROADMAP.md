# Roadmap and Milestone Gates

Milestones are sequential gates, not calendar promises. A later milestone may be researched early, but production implementation should not bypass the prior gate.

## M0 — Foundation and feasibility

Outcome: a runnable repository, agreed domain/event model, and documented provider constraints.

Gate: local frontend, Rust service, and database run with one command; canonical event model is reviewed; data-source feasibility is recorded.

Tickets: FT-001–FT-003.

## M1 — Simulated operations console

Outcome: a convincing end-to-end console using deterministic replay data.

Gate: a recorded flight scenario appears on the map and board, streams through Rust, and can be replayed identically.

Tickets: FT-101–FT-104.

## M2 — Live weather and hazard intelligence

Outcome: current NOAA aviation weather is normalized and correlated with routes to produce explainable alerts.

Gate: live and fixture-based SIGMET processing passes geometry, freshness, deduplication, and degraded-feed tests.

Tickets: FT-201–FT-204.

## M3 — Commercial flight data and operational workflow

Outcome: licensed flight data and a complete dispatcher alert lifecycle operate behind authentication and tenant boundaries.

Gate: provider agreement permits the use case; source health is visible; audit and permissions pass review.

Tickets: FT-301–FT-304.

## M4 — Pilot readiness and operational hardening

Outcome: the system can support a limited, advisory-only evaluation with real operations users.

Gate: security review, failure drills, usability validation, runbooks, and measurable pilot criteria are complete.

Tickets: FT-401–FT-403.

## M5 — Optimization research and controlled recommendations

Outcome: determine whether route, altitude, or message recommendations can be validated safely and economically.

Gate: offline recommendations beat a documented baseline on held-out historical cases, expose evidence, and remain human-approved. This gate does not itself authorize operational use.

Tickets: FT-501–FT-503.
