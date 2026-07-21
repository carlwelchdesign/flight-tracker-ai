# Replay Scenario Contract

FT-101 provides deterministic development data without creating a simulation-only downstream architecture. Replay and future provider adapters publish the same `NormalizedEventBatch`: one immutable raw `ProviderEnvelope` plus its normalized canonical events.

## Version 1 format

Scenario files are JSON and must contain:

- `schema_version`: currently `1`.
- `id`: stable scenario/feed name.
- `namespace_id`: fixed UUID namespace used to derive repeatable envelope and position IDs.
- `operator_id`: tenant owning every emitted fact.
- `start_time`: fixed RFC 3339 UTC virtual-clock origin.
- `flights`: flight metadata and a scenario-only `role` of `normal`, `delayed`, or `hazard_adjacent`.
- `events`: strictly ordered records with a unique `sequence`, a nondecreasing `offset_ms`, a stable `provider_record_id`, and a tagged payload.

Supported payload tags are `flight_snapshot`, `position`, `planned_route`, and `weather_hazard`. Planned routes reference a known flight, carry a positive route version and effective window, and contain at least two ordered WGS84 points. Payload measurements use the canonical model's explicit units, references, WGS84 coordinate names, and snake-case enum values. Hazard polygons require at least four points and must repeat the first point as the last point.

The milestone fixture is [m1-operations-v1.json](../fixtures/replay/m1-operations-v1.json). Copy it when authoring a scenario, replace every stable identity, keep events sorted by `(offset_ms, sequence)`, and run `cargo test --workspace` to validate the file and normalization.

FT-203 adds [m2-route-hazard-v1.json](../fixtures/replay/m2-route-hazard-v1.json), which emits a versioned planned route and hazard through the same canonical replay boundary for deterministic correlation-rule testing.

## Deterministic behavior

- Virtual time starts at `start_time` and advances only while running.
- Speed uses exact integer ratios: `0.25x`, `0.5x`, `1x`, `2x`, `4x`, or `8x`.
- Envelope and position IDs are UUID v5 values derived from the scenario namespace and event sequence.
- Raw-payload hashes are SHA-256 values over the normalized JSON payload.
- Event, received, and processed timestamps are fixed virtual timestamps, never wall-clock timestamps.
- Reset pauses the scenario and restores its cursor and virtual elapsed time to zero. Resume then emits the same normalized batches as the first run.

## Development controls

The local Compose stack opts in with `APP_ENV=development`, `ENABLE_REPLAY_CONTROLS=true`, and an explicit `REPLAY_SCENARIO_PATH`. The API exposes:

- `GET /api/dev/replay`
- `POST /api/dev/replay/pause`
- `POST /api/dev/replay/resume`
- `POST /api/dev/replay/reset`
- `POST /api/dev/replay/speed` with JSON such as `{ "speed": "4x" }`
- `POST /api/dev/replay/outage` with `{ "active": true }` to suspend source events and `{ "active": false }` to restore them

The scenario starts paused. Status reports phase, speed, event cursor, emitted and total event counts, virtual elapsed milliseconds, virtual time, and `feed_outage`. A simulated outage leaves the replay and projection workers healthy but freezes virtual source progress. This intentionally distinguishes provider/source degradation from a crashed critical worker. Reset clears the outage.

## Production safety boundary

Replay is disabled by default, so no control routes are mounted. Enabling it requires all three development settings above. Startup fails before connecting to the database when replay is requested with `APP_ENV=production`, an unknown environment, an invalid toggle, or no scenario path. Production deployments must omit replay variables or explicitly set `ENABLE_REPLAY_CONTROLS=false`.
