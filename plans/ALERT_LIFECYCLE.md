# Alert Ranking and Lifecycle

FT-204 turns a deterministic FT-203 route–hazard match into a durable, human-managed dispatcher alert. The rule, ranking policy, SQL store, ingestion worker, HTTP adapter, and React queue are separate boundaries so policy can be tested without Postgres or Axum and lifecycle writes stay transactional.

## Versioned ranking

Attention score version `1` is a transparent 0–100 sum:

| Component | Points |
| --- | ---: |
| Hazard severity | unknown 10; advisory 25; significant 45; severe 60 |
| Horizontal relation | intersection 25; within-margin 10–20 based on distance |
| Altitude relation | overlap 10; otherwise 0 |
| Time urgency | 5 when validity ends within 30 minutes; otherwise 0 |

Score bands are information 0–29, advisory 30–59, warning 60–84, and critical 85–100. The stored evidence contains every component, score version, rule evidence, rule version, route version, hazard revision, proximity margin, and closest approach.

Only `match` decisions become candidates. Clear and indeterminate decisions are suppressed rather than represented as actionable alerts.

## Identity and supersession

- `series_key` identifies one operator, flight, and provider hazard series.
- `dedupe_key` adds material route version, hazard revision, rule version, and urgency bucket.
- Reprocessing the same key is idempotent and returns the existing alert.
- New material evidence creates a new alert revision with `supersedes_alert_id`; it never overwrites prior evidence.
- A prior open or acknowledged revision is resolved in the same transaction and receives a system-authored append-only resolve action.
- Advisory transaction locks serialize candidate creation per series and dispatcher actions per idempotency key.

## Human lifecycle

Valid transitions are:

- open → acknowledged, dismissed, or resolved;
- acknowledged → dismissed or resolved;
- comment → no lifecycle change and is allowed in any state;
- dismissed and resolved are terminal.

Dismissal requires a non-empty reason. Every accepted command inserts an `alert_actions` row in the same transaction as the lifecycle update. Client-provided idempotency keys make retries safe and cannot be reused against another alert.

## HTTP contract

- `GET /api/alerts?operator_id={uuid}` returns only current open/acknowledged series revisions, ordered by lifecycle, attention descending, oldest event, then ID.
- `GET /api/alerts?operator_id={uuid}&include_terminal=true` also returns current terminal revisions.
- `GET /api/alerts/{id}?operator_id={uuid}` returns stored evidence and ordered audit actions.
- `POST /api/alerts/{id}/actions` accepts `operator_id`, `action`, `actor_id`, `idempotency_key`, and optional `comment`.

Operator ID is mandatory on every read and mutation until authenticated tenant context replaces the explicit development boundary.

## Replay and dispatcher interface

The replay alert worker persists simulation envelopes, flights, positions, routes, and hazards through the canonical PostGIS schema before alert creation. The M1 scenario includes a versioned route for the hazard-adjacent flight, allowing a normal local run to produce a real persisted alert.

The dispatcher queue exposes explicit loading, empty, unavailable, selected, and action-pending states. It displays the score breakdown, rule/evidence versions, distance and margin, lifecycle, and append-only audit history. Acknowledge, comment, resolve, and dismiss remain explicit human actions; nothing sends an operational instruction.

## Verification

- Pure lifecycle and ranking threshold tests.
- PostGIS integration coverage for ranking order, exact dedupe, evidence supersession, automatic resolution, idempotent acknowledgement, comments, dismissal reasons, terminal suppression, and history.
- Typed frontend parser tests and dispatcher interaction tests.
- Strict Clippy, Rust format/test/release build, web lint/typecheck/test/production build, schema migration, and live replay/API checks.
