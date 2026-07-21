# Canonical Aviation Event Model

Contract version: 1

This document defines the provider-independent boundary introduced by FT-002.
The Rust contract is authoritative for application data; the PostgreSQL/PostGIS
migration is authoritative for persisted invariants.

## Data flow and responsibility

```text
provider payload -> ProviderEnvelope -> provider adapter -> CanonicalEvent -> projection/rules
```

- `ProviderEnvelope` owns immutable provider JSON, its SHA-256 hash, provider
  identity, optional provider record ID, and ingestion timestamps.
- Normalized facts carry `SourceAttribution`, which points back to the envelope
  without copying provider-specific JSON into the domain.
- `CanonicalEvent` is a tagged, versioned enum for normalized facts. It does not
  contain `ProviderEnvelope`; raw inputs and normalized outputs are separate
  contracts.
- Axum, SQLx, provider clients, and UI models remain outside the domain module.

## Entity and table map

| Rust type | PostgreSQL table | Identity and source |
| --- | --- | --- |
| `ProviderEnvelope` | `provider_envelopes` | Envelope UUID, operator UUID, provider/feed/record ID |
| `AirportObservation` | `airport_observations` | Observation UUID, station, operator/source-envelope UUIDs |
| `Flight` | `flights` | Flight UUID, operator UUID, source-envelope UUID |
| `AircraftPosition` | `aircraft_positions` | Position UUID, operator/flight/source-envelope UUIDs |
| `PlannedRoute` | `planned_routes` | Route UUID, operator/flight/source-envelope UUIDs, route version |
| `WeatherHazard` | `weather_hazards` | Hazard UUID, operator/source-envelope UUIDs |
| `Alert` | `alerts` and `alert_evidence` | Alert UUID, operator UUID, series/revision, material dedupe key, evidence envelopes |
| `AlertAction` | `alert_actions` | Action UUID, operator/alert UUIDs, human actor and idempotency key |
| `SourceHealth` | `source_health` | Health UUID, operator UUID, provider/feed identity |
| Retention policy/run | `retention_policies`, `retention_runs` | Tenant/data-class/provider policy version, requester/approver/executor, cutoff and counts |
| Raw deletion tombstone | `data_deletion_tombstones` | Tenant/provider/feed/raw hash and deletion-run evidence |
| Lifecycle deletion tombstone | `lifecycle_deletion_tombstones` | Tenant/data-class/source-record identity plus deletion/minimization evidence |

Every operational table includes a non-null `operator_id`. Composite foreign
keys include `operator_id`, so a record cannot reference an envelope, flight,
hazard, or alert belonging to another operator even if application scoping fails.

Provider envelope source identity and hash evidence remain stable. An approved
raw-payload retention run may replace `raw_payload` with an empty object and
attach a tombstone; the insert/update trigger prevents an identical deleted
payload from being restored into the tenant.

Approved application-lifecycle runs may delete old authorization events and
expired session revocations or minimize an exclusively tenant-owned inactive
identity. Typed lifecycle tombstones suppress restored audit/revocation rows and
force restored identity mappings back to their minimized state.

## Versioning

- Version numbers are positive integers; zero is rejected by Rust and database
  constraints.
- Version 1 is `SchemaVersion::V1`.
- Additive compatible fields may remain in the same version when existing
  consumers safely ignore them. Renames, semantic changes, unit changes, or
  required-field changes require a new version and migration plan.
- Historical envelopes and facts retain the version under which they were
  interpreted; they are not silently re-labeled after adapter changes.

## Time conventions and nullability

All timestamps are UTC instants. Rust serializes them as RFC 3339 strings ending
in `Z`; PostgreSQL stores them as `TIMESTAMPTZ`.

| Field | Meaning | Nullability |
| --- | --- | --- |
| `event_time` | Time assigned by the provider or originating rule/event | Nullable only on raw envelopes because some payloads omit it |
| `received_at` | Time this system accepted the raw record or normalized input | Required |
| `processed_at` | Time normalization completed | Nullable on an unprocessed/failed envelope; required on normalized facts |
| `scheduled_*` | Provider schedule values | Nullable because unscheduled operations or incomplete feeds may omit them |
| `effective_to` | End of a route version | Nullable to represent the currently effective version |
| `last_success_at` | Most recent successful source interaction | Nullable before the first success |

`processed_at` cannot precede `received_at`. Event time is not required to
precede receive time because provider clock error and future schedule events are
valid observations that must remain visible rather than be silently rewritten.

## Geometry and units

- Coordinates use WGS84 / EPSG:4326.
- GeoJSON and PostGIS coordinate order is **longitude, latitude** (`x`, then
  `y`). Rust uses named `longitude_degrees` and `latitude_degrees` fields and
  validates longitude to `[-180, 180]` and latitude to `[-90, 90]`.
- Aircraft positions are PostGIS `POINT`; planned routes are `LINESTRING`;
  weather-hazard footprints are `POLYGON`. Future multipolygon support requires
  a versioned contract decision rather than an implicit geometry-type change.
- Altitude always includes numeric value, unit (`feet` or `meters`), and
  reference (`mean_sea_level`, `above_ground_level`, `flight_level`, or
  `ellipsoid`). A missing altitude is null as a whole, never a guessed zero.
- Ground speed always includes its unit (`knots` or `kilometers_per_hour`).
- Heading is true degrees in the half-open range `[0, 360)`.
- Source delay and stale thresholds are integer seconds and use `_seconds`
  field/column names.

## Persistence behavior

- Raw payloads are JSONB and normalized facts are relational/spatial rows.
- Provider record IDs are optional and may be reused by a provider for revised
  messages. The same operator/provider/feed/record ID plus payload hash is
  unique to make identical delivery retries idempotent without suppressing a
  changed payload.
- NOAA SIGMET revisions share a stable external series identity. Each changed payload creates a new immutable envelope and hazard revision with an explicit superseded record and active/cancelled status.
- Malformed live-provider records retain their raw envelope and an `ingestion_failures` quarantine row; only successfully normalized records enter canonical event streams.
- Source attribution is mandatory on external normalized facts.
- Alert evidence is an ordered join to provider envelopes rather than an opaque
  JSON list, preserving tenant constraints and queryability.
- Alert actions are append-only at the application boundary; later workflow
  tickets will add authorization and mutation operations without rewriting the
  historical action rows.

## Deliberate non-goals for FT-002

- Provider payload schemas and normalization adapters
- Current-state projection queries and repositories
- Authentication/authorization policy beyond the persisted operator boundary
- Multipolygon and 3D spatial-volume operations
- Alert eligibility, severity rules, and lifecycle commands
