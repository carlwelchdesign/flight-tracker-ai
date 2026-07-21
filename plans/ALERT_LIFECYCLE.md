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

Assignment is a workflow action rather than a lifecycle transition. Open and acknowledged alerts may be assigned to an active dispatcher, operator, or administrator in the authenticated tenant. Each accepted human action increments `workflow_version`; clients must submit the version they reviewed, and stale writes receive `409 alert_conflict` before any audit or alert row is changed.

Dismissal uses one structured reason: duplicate alert, stale source data, incorrect correlation, not operationally relevant, or other. `other` also requires a note. Every accepted command inserts an `alert_actions` row in the same transaction as the lifecycle, assignment, and workflow-version update. Client-provided idempotency keys make retries safe and cannot be reused against another alert.

## HTTP contract

- `GET /api/alerts` returns the authenticated tenant's current open/acknowledged series revisions, ordered by lifecycle, attention descending, oldest event, then ID. The bounded result defaults to 200 and caps at 500.
- `GET /api/alerts` accepts exact severity and status filters, a callsign or flight UUID, inclusive event-time bounds, and an assignee identity UUID or `unassigned`. An explicit terminal status includes that status without requiring `include_terminal`.
- `GET /api/alerts/assignees` returns only active tenant members who can manage alerts.
- `GET /api/alerts/{id}` returns stored evidence, assignment, workflow version, and ordered audit actions.
- `POST /api/alerts/{id}/actions` accepts `action`, `idempotency_key`, `expected_workflow_version`, and action-specific comment, assignee, or dismissal-reason fields. Operator and actor authority come only from the authenticated Rust context.

Every read and mutation is tenant-scoped from the authenticated context. Browser-provided operator or actor identifiers are not part of the contract.

## Replay and dispatcher interface

The replay alert worker persists simulation envelopes, flights, positions, routes, and hazards through the canonical PostGIS schema before alert creation. The M1 scenario includes a versioned route for the hazard-adjacent flight, allowing a normal local run to produce a real persisted alert.

The dispatcher queue exposes explicit loading, filtered-empty, unavailable, partial assignment-directory failure, selected, action-pending, success, validation, and concurrent-update recovery states. It displays the score breakdown, rule/evidence versions, distance and margin, lifecycle, assignment, and append-only audit history before controls. Acknowledge, assign, comment, resolve, and dismiss remain explicit human actions; nothing sends an operational instruction.

## Verification

- Pure lifecycle and ranking threshold tests.
- PostGIS integration coverage for ranking order, exact dedupe, evidence supersession, automatic resolution, idempotent acknowledgement, tenant-safe assignment, optimistic conflicts, structured dismissal, terminal suppression, filtering, and a 100-of-120 bounded volume page.
- Typed frontend parser tests and dispatcher interaction tests, including all five filter dimensions, assignment, conflict recovery, structured dismissal, and a 150-alert queue.
- Strict Clippy, Rust format/test/release build, web lint/typecheck/test/production build, schema migration, and live replay/API checks.
