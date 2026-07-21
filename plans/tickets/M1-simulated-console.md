# M1 — Simulated Operations Console

Default owner: Full-stack engineering, with product-design review for dispatcher workflows.

## FT-101 — Build deterministic replay infrastructure

Status: Complete

Branch: `feat/ft-101-replay-infrastructure`
Final implementation commit: `47a8029`
Pull request: [#5](https://github.com/carlwelchdesign/flight-tracker-ai/pull/5) (merged at `efc2cf6`)
Owner: Full-stack engineering

Create a versioned scenario format and virtual clock that emits positions and operational events through the same path used by live providers.

Dependencies: FT-001, FT-002

Acceptance checklist:

- [x] At least one multi-flight scenario includes normal, delayed, and hazard-adjacent flights.
- [x] Pause, resume, speed, reset, and deterministic restart are supported in development.
- [x] Replaying identical fixtures produces identical normalized events.
- [x] Replay controls cannot be enabled accidentally in production.
- [x] Scenario schema and authoring instructions are documented.

Verification evidence: `fixtures/replay/m1-operations-v1.json`; `plans/REPLAY_SCENARIOS.md`; `cargo fmt --all --check`; strict workspace Clippy; 18 passing Rust tests; production Rust release build; web install, audit, lint, typecheck, and production build; Compose configuration validation; live Compose API sequence covering status, 8x speed, resume, pause, and reset; production startup rejected `ReplayControlsForbidden`; implementation commit `47a8029`; PR [#5](https://github.com/carlwelchdesign/flight-tracker-ai/pull/5), with Rust, web, and API/PostGIS smoke checks passing.

## FT-102 — Implement fleet API and live event stream

Status: Complete

Branch: `feat/ft-102-fleet-api-event-stream`
Final implementation commit: `7e99083`
Pull request: [#6](https://github.com/carlwelchdesign/flight-tracker-ai/pull/6) (merged at `aed432d`)
Owner: Backend engineering

Project current flight state from replay events and expose list, detail, timeline, and SSE endpoints.

Dependencies: FT-101

Acceptance checklist:

- [x] Flight list and detail endpoints return typed, paginated responses.
- [x] Timeline returns source-attributed operational events in stable order.
- [x] SSE reconnect behavior and event IDs are tested.
- [x] Invalid or out-of-order events do not corrupt current state.
- [x] API latency and stream connection metrics are emitted.

Verification evidence: `plans/FLEET_API.md`; focused projection, HTTP, SSE, metrics, replay-reset, and replay-to-public-API tests; 31 passing Rust tests; strict workspace Clippy; Rust release build; web dependency audit, lint, typecheck, and production build; Compose configuration and diff hygiene; implementation commit `7e99083`; PR [#6](https://github.com/carlwelchdesign/flight-tracker-ai/pull/6), with Rust, web, and API/PostGIS smoke checks passing.

## FT-103 — Build map, flight board, and flight detail experience

Status: Complete

Branch: `feat/ft-103-operations-console`
Final implementation commit: `160f5f6`
Pull request: [#7](https://github.com/carlwelchdesign/flight-tracker-ai/pull/7) (merged at `18a5a23`)
Owner: Frontend engineering with product-design review

Create the desktop operations interface with synchronized map and table selection.

Dependencies: FT-102

Acceptance checklist:

- [x] Board shows callsign, route, phase, schedule variance, freshness, and attention level.
- [x] Map shows aircraft, route, origin, and destination with accessible selection behavior.
- [x] Selecting a flight synchronizes the map, board, and detail panel.
- [x] Loading, empty, disconnected, stale, and error states are designed and implemented.
- [x] Keyboard navigation and basic screen-reader labels are verified.
- [x] Dense layouts remain usable at the agreed minimum desktop viewport.

Verification evidence: `plans/OPERATIONS_CONSOLE.md`; 31 passing Rust tests; strict workspace Clippy; Rust formatting and release build; 5 passing frontend interaction, stale-data, collision, empty-state, and payload-validation tests; dependency audit with 0 vulnerabilities; frontend lint, typecheck, and production build; Compose configuration and diff hygiene; browser-verified live, disconnected, empty, timeline, replay-control, pointer-selection, and synchronized-selection behavior; no horizontal page overflow at 1440x900, the agreed 1180x720 minimum, 820x900, or 390x844; implementation commit `160f5f6`; PR [#7](https://github.com/carlwelchdesign/flight-tracker-ai/pull/7), with Rust, web, and API/PostGIS smoke checks passing.

## FT-104 — Add source health and operational observability

Status: Complete

Branch: `feat/ft-104-source-health-observability`
Final implementation commit: `c03c4f0`
Pull request: [#8](https://github.com/carlwelchdesign/flight-tracker-ai/pull/8) (merged at `da1a6ad`)
Owner: Full-stack engineering

Make replay and service health visible to operators and developers.

Dependencies: FT-102, FT-103

Acceptance checklist:

- [x] UI shows last event time, last received time, and connection state.
- [x] Rust service emits structured logs with correlation IDs.
- [x] Health and readiness reflect database and critical-worker state.
- [x] A simulated feed outage produces an obvious degraded UI state.
- [x] A short troubleshooting runbook is documented.

Verification evidence: `plans/OPERATIONS_RUNBOOK.md`; focused worker-health, correlation-ID, JSON-log, replay-outage, public-route, health, readiness, proxy-parser, timing, badge, outage, and degraded-worker tests; 34 passing Rust library tests, 3 binary tests, and the schema contract test; strict workspace Clippy; Rust formatting and release build; 11 passing frontend tests; dependency audit, lint, typecheck, and production build; Compose configuration and diff hygiene; browser-verified distinct event and receipt times, healthy service/stream state, simulated outage and recovery, degraded/reconnecting state with the last accepted picture retained, and no horizontal overflow at the agreed 1180x720 minimum; implementation commit `c03c4f0`; PR [#8](https://github.com/carlwelchdesign/flight-tracker-ai/pull/8), with Rust, web, and API/PostGIS smoke checks passing.
