# Roadmap and Milestone Gates

Milestones are sequential gates, not calendar promises. A later milestone may be researched early, but production implementation should not bypass the prior gate.

## M0 — Foundation and feasibility

Outcome: a runnable repository, agreed domain/event model, and documented provider constraints.

Gate: local frontend, Rust service, and database run with one command; canonical event model is reviewed; data-source feasibility is recorded.

Tickets: FT-001–FT-005.

## M1 — Simulated operations console

Outcome: a convincing end-to-end console using deterministic replay data.

Gate: a recorded flight scenario appears on the map and board, streams through Rust, and can be replayed identically.

Tickets: FT-101–FT-104.

## M2 — Live weather and hazard intelligence

Outcome: current NOAA aviation weather is normalized and correlated with routes to produce explainable alerts.

Gate: live and fixture-based SIGMET processing passes geometry, freshness, deduplication, and degraded-feed tests.

Tickets: FT-201–FT-204.

## M3 — Portfolio live data and operational workflow

Outcome: an eligible free, best-effort flight-position source can feed the same canonical boundary as replay while the complete alert workflow operates behind authentication and tenant boundaries.

Gate: official source terms permit public non-commercial portfolio display; attribution, rate limits, freshness, degraded behavior, and replay fallback are visible; audit and permissions pass review.

Tickets: FT-301–FT-304.

## M4 — Portfolio launch and demonstration hardening

Outcome: recruiters and hiring managers can use a reliable hosted demonstration without mistaking it for an operational aviation system.

Gate: public-demo security review, failure/fallback checks, recruiter-oriented usability validation, deployment runbooks, and end-to-end hosted smoke checks are complete.

Tickets: FT-401–FT-404.

## M4.1 — Public decision intelligence and exploration

Outcome: the public tracker explains why a demonstration flight needs
attention, lets a viewer replay and inspect its motion, supports direct search
and shareable views, and adds source-attributed airport forecast and pilot-report
context.

Gate: FT-403 and FT-404 are complete first; then a neutral reviewer can use the
attention explanation, deterministic time machine, telemetry charts, direct
links, TAF timeline, and nearby PIREPs without facilitator help. Public data
remains sanitized and read-only, live ADS-B history remains ephemeral, and
protected operational controls remain behind authentication.

Tickets: FT-413–FT-416. See
[`tickets/M4.1-public-decision-intelligence.md`](tickets/M4.1-public-decision-intelligence.md).

## M5 — Optimization research and controlled recommendations

Outcome: determine whether route, altitude, or message recommendations can be validated safely and economically.

Gate: offline recommendations beat a documented baseline on held-out historical cases, expose evidence, and remain human-approved. This gate does not itself authorize operational use.

Tickets: FT-501–FT-503. M5 begins after the M4.1 public product sequence so
recommendations build on an already understandable evidence and replay model.

## Optional future production track

Commercial flight-data procurement, contractual SLAs, real-operator trials, production certification, and operational support are intentionally outside the portfolio roadmap. The completed procurement research remains available if the project is ever re-scoped, but none of it blocks M3 or M4.
