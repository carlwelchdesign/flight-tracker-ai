# Architecture

## Recommended shape

Use a modular monolith first: one Rust workspace containing the API, ingestion workers, domain model, and alert engine, backed by PostgreSQL/PostGIS. Keep the Next.js interface separate. Split services only after load or organizational boundaries justify it.

## Technology choices

### Frontend

- Next.js with TypeScript
- MapLibre GL for map rendering
- Server-Sent Events for one-way live updates; adopt WebSockets only if the product needs sustained two-way sessions
- A query/cache library for server state
- Accessible, dense desktop-first operations UI

### Rust backend

- Axum for HTTP APIs and SSE
- Tokio for asynchronous ingestion and jobs
- SQLx for compile-time-checked PostgreSQL access
- Serde for typed provider payloads and canonical events
- Tracing and OpenTelemetry-compatible instrumentation
- Geo/geo-types for application geometry; PostGIS remains authoritative for spatial storage and complex queries
- Reqwest for provider clients
- Thiserror/anyhow with domain errors at public boundaries

### Storage and jobs

- PostgreSQL with PostGIS
- Redis only when durable queueing, fan-out, or rate coordination is actually needed
- Object storage for large raw payload archives or replay fixtures
- Provider payloads stored separately from normalized domain facts
- Malformed provider records retained with explicit quarantine evidence

## Service boundaries

| Component | Responsibility |
|---|---|
| Web app | Dispatcher experience, maps, tables, review actions, freshness UI |
| API | Authenticated reads/writes, validation, SSE subscriptions |
| Ingestion workers | Poll/stream providers, apply each source's retention boundary, and normalize canonical events |
| Alert engine | Deterministic rule evaluation, deduplication, severity, evidence |
| Replay runner | Re-emits time-ordered fixtures using the same normalization and rule paths |
| PostgreSQL/PostGIS | Operational state, history, geometry, rule results, audit log |

## Canonical data flow

`provider payload -> raw envelope -> normalized event -> current projection -> rule evaluation -> alert -> human action -> audit event`

Every envelope should carry:

- Provider and provider record ID
- Event time, received time, and processed time
- Schema version
- Raw-payload reference or hash
- Correlation identifiers such as flight, aircraft, airport, or hazard ID
- Quality/freshness state

ADSB.lol is the deliberate ephemeral exception to the normal raw-envelope
retention path. Its per-record envelope exists only while the in-memory Rust
adapter normalizes and publishes a current-state batch; raw and normalized
ADSB.lol records are not written to PostgreSQL, fixtures, logs, exports,
analytics, backups, or an LLM. The fleet API and Next.js proxy preserve
`Cache-Control: no-store`. See `ADSBLOL_INTEGRATION.md`.

## Initial domain entities

- `Flight` — operational identity, callsign, origin, destination, schedule, status
- `AircraftPosition` — timestamped point, altitude, heading, speed, source quality
- `PlannedRoute` — versioned line geometry and effective time
- `WeatherHazard` — versioned polygon/volume, altitude band, severity, validity window
- `AirportObservation` — METAR/TAF and derived operational fields
- `Alert` — type, severity, lifecycle, rule version, evidence, dedupe key
- `AlertAction` — acknowledgement, dismissal, comment, resolution, actor, timestamp
- `SourceHealth` — provider status, last success, delay, error, stale threshold
- `ReplayScenario` — immutable fixture manifest and virtual clock configuration

## API outline

- `GET /health` and `GET /readiness` for minimal public load-balancer probes
- `GET /api/system/health` and `GET /api/system/readiness` for authenticated diagnostics
- `GET /api/flights`
- `GET /api/flights/{id}`
- `GET /api/flights/{id}/timeline`
- `GET /api/hazards`
- `GET /api/alerts`
- `GET /api/alerts/{id}`
- `POST /api/alerts/{id}/actions`
- `GET /api/source-health`
- `GET /api/live-positions/status`
- `GET /api/events/stream`
- Development-only replay controls protected by environment and authorization

Public `/health` is an HTTP 200 liveness probe with exactly `{"status":"ok"}`. Public `/readiness` is fail-closed and returns only `ready` or `not_ready`; it returns 503 unless the database, PostGIS, and all registered critical workers are ready. Authenticated `/api/system/health` and `/api/system/readiness` expose the service version, critical-worker state/heartbeat, and named database/PostGIS checks to operators. A worker is degraded while starting, after a failed/stopped task, or when its heartbeat exceeds the health threshold.

## Security and operational constraints

- Tenant or operator ID must exist on operational records before real customer data is introduced.
- Provider credentials remain server-side and are never exposed to the browser.
- Audit events are append-only at the application layer.
- Inputs are untrusted, including provider payloads.
- External data must expose freshness; stale data must not silently look current.
- Recommendations and generated messages must include evidence and approval status.

## Why Rust

Rust is appropriate for long-running network clients, typed normalization, concurrency, geospatial rule evaluation, and resource-efficient workers. Its main cost is development speed and ecosystem familiarity. The modular-monolith approach limits that cost: use Rust for the operational core and avoid premature microservices. Introduce Python later only for numerical research that clearly benefits from mature scientific libraries.
