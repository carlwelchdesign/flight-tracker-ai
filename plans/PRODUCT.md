# Product Definition

## Product thesis

Dispatchers lose time and attention by switching between flight tracking, weather, airport, airspace, and communication systems. This product should create one explainable operational picture and surface only the events that materially affect monitored flights.

The repository is a non-commercial portfolio project. Its purpose is to demonstrate product design, full-stack engineering, Rust backend architecture, geospatial reasoning, data provenance, and operational UX to recruiters and hiring managers. It must never imply that it is approved for real flight operations.

## Primary audience

The first viewer is a recruiter or hiring manager exploring a convincing dispatcher-style workflow. The interface models an airline or charter dispatcher monitoring a modest fleet, but no real operator, contract, or operational deployment is required for the portfolio release.

## Primary job to be done

“Show me which active flights need attention, why they need attention, how fresh the evidence is, and what action has already been taken.”

For the public portfolio experience, that job must be understandable without
sign-in: a viewer should be able to select a demonstration flight, inspect why
it needs attention, replay the evidence change over time, and share the exact
view. Authentication remains necessary only for protected operational actions,
tenant evidence, and audit history.

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

- Real operational use or safety certification
- Commercial flight-data procurement or contractual uptime guarantees
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
| Public explanation comprehension | A neutral reviewer identifies the attention flight and explains the contributing evidence without facilitator help |
| Replay comprehension | A neutral reviewer finds when the attention state changed using the time machine and telemetry charts |
| Shareable demonstration | A direct URL restores the intended region, scenario or aircraft, replay time, and visible layers |
| Airport weather comprehension | A neutral reviewer distinguishes a current METAR, TAF forecast, and nearby PIREP |

## Product gates

- Any free live-data source must have documented official terms permitting the exact public, non-commercial portfolio use, including hosting, display, storage, and attribution.
- Live aircraft data is optional and best-effort. Deterministic replay remains the guaranteed demo path.
- Simulated, live, stale, and unavailable data must be labeled clearly; scenario metadata must never be presented as provider-supplied fact.
- Real dispatcher or pilot messages remain draft-only until identity, permissions, audit, and human approval are implemented.
- Optimization work requires historical truth data and an agreed validation method.

Commercial provider rights, pricing, trials, SLAs, and real-operator validation are preserved as an optional production track. They do not block the portfolio MVP.
