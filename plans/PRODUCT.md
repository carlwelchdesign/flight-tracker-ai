# Product Definition

## Product thesis

Dispatchers lose time and attention by switching between flight tracking, weather, airport, airspace, and communication systems. This product should create one explainable operational picture and surface only the events that materially affect monitored flights.

## Beachhead user

The first user is an airline or charter-operation dispatcher monitoring a modest fleet during day-of-operations. A demo user may be an operations manager evaluating whether the product can reduce alert fatigue and improve response time.

## Primary job to be done

“Show me which active flights need attention, why they need attention, how fresh the evidence is, and what action has already been taken.”

## MVP workflow

1. Dispatcher opens the operations console.
2. Flight board ranks monitored flights by operational attention level.
3. Map shows aircraft, routes, airports, and active hazard polygons.
4. A deterministic engine detects route/hazard interactions and creates an explainable alert.
5. Dispatcher inspects the underlying source data.
6. Dispatcher acknowledges, dismisses, comments on, or resolves the alert.
7. The system records the action in an audit trail.
8. A replay mode reproduces the same scenario for demonstrations and regression tests.

## MVP capabilities

- Simulated and replayable fleet position stream
- Flight board and interactive map
- Flight detail and event timeline
- NOAA weather products, beginning with SIGMETs and METARs
- Route–hazard intersection detection
- Severity, deduplication, acknowledgement, resolution, and audit trail
- Data-source freshness and degraded-state indicators
- Read-only operational roles initially; controlled write permissions before real deployments

## Hard non-goals for the MVP

- Autonomous dispatch decisions
- Certified flight planning
- Pilot-facing route or altitude commands
- Tail-specific fuel optimization
- Passenger re-accommodation
- Full global NOTAM normalization
- ACARS, SITA, or ARINC production integration
- An LLM making eligibility, severity, or safety calculations

## Success measures

| Measure | MVP target |
|---|---|
| Demo scenario reproducibility | Same inputs generate the same alerts every run |
| Alert explainability | Every alert exposes rule version, source, timestamps, and geometry |
| Data freshness visibility | Every external layer shows last successful update and stale state |
| Detection latency | Hazard update to alert creation measured and reported; initial target under 60 seconds |
| Workflow completeness | Dispatcher can acknowledge, comment, dismiss, and resolve an alert |
| False-positive learning | Dismissal reason is captured for later rule tuning |

## Product gates

- A commercial flight-data provider is not selected until licensing, coverage, latency, and cost are documented.
- Real dispatcher or pilot messages remain draft-only until identity, permissions, audit, and human approval are implemented.
- Optimization work requires historical truth data and an agreed validation method.
