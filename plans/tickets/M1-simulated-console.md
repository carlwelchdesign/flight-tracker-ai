# M1 — Simulated Operations Console

Default owner: Full-stack engineering, with product-design review for dispatcher workflows.

## FT-101 — Build deterministic replay infrastructure

Status: Not started

Branch: `feat/ft-101-replay-infrastructure`
Final commit: Pending
Pull request: Pending

Create a versioned scenario format and virtual clock that emits positions and operational events through the same path used by live providers.

Dependencies: FT-001, FT-002

Acceptance checklist:

- [ ] At least one multi-flight scenario includes normal, delayed, and hazard-adjacent flights.
- [ ] Pause, resume, speed, reset, and deterministic restart are supported in development.
- [ ] Replaying identical fixtures produces identical normalized events.
- [ ] Replay controls cannot be enabled accidentally in production.
- [ ] Scenario schema and authoring instructions are documented.

Verification evidence: Pending.

## FT-102 — Implement fleet API and live event stream

Status: Not started

Branch: `feat/ft-102-fleet-api-event-stream`
Final commit: Pending
Pull request: Pending

Project current flight state from replay events and expose list, detail, timeline, and SSE endpoints.

Dependencies: FT-101

Acceptance checklist:

- [ ] Flight list and detail endpoints return typed, paginated responses.
- [ ] Timeline returns source-attributed operational events in stable order.
- [ ] SSE reconnect behavior and event IDs are tested.
- [ ] Invalid or out-of-order events do not corrupt current state.
- [ ] API latency and stream connection metrics are emitted.

Verification evidence: Pending.

## FT-103 — Build map, flight board, and flight detail experience

Status: Not started

Branch: `feat/ft-103-operations-console`
Final commit: Pending
Pull request: Pending

Create the desktop operations interface with synchronized map and table selection.

Dependencies: FT-102

Acceptance checklist:

- [ ] Board shows callsign, route, phase, schedule variance, freshness, and attention level.
- [ ] Map shows aircraft, route, origin, and destination with accessible selection behavior.
- [ ] Selecting a flight synchronizes the map, board, and detail panel.
- [ ] Loading, empty, disconnected, stale, and error states are designed and implemented.
- [ ] Keyboard navigation and basic screen-reader labels are verified.
- [ ] Dense layouts remain usable at the agreed minimum desktop viewport.

Verification evidence: Pending.

## FT-104 — Add source health and operational observability

Status: Not started

Branch: `feat/ft-104-source-health-observability`
Final commit: Pending
Pull request: Pending

Make replay and service health visible to operators and developers.

Dependencies: FT-102, FT-103

Acceptance checklist:

- [ ] UI shows last event time, last received time, and connection state.
- [ ] Rust service emits structured logs with correlation IDs.
- [ ] Health and readiness reflect database and critical-worker state.
- [ ] A simulated feed outage produces an obvious degraded UI state.
- [ ] A short troubleshooting runbook is documented.

Verification evidence: Pending.
