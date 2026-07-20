# M3 — Commercial Data and Operational Workflow

Default owner: Product for provider contracting; engineering and security for implementation.

## FT-301 — Select and contract a commercial flight-data provider

Status: Not started

Branch: `docs/ft-301-commercial-provider-selection`
Final commit: Pending
Pull request: Pending

Resolve the provider feasibility work with a documented selection appropriate for the intended use.

Dependencies: FT-003

Acceptance checklist:

- [ ] Commercial display, storage, derived-data, and customer-use rights are confirmed.
- [ ] Coverage and latency are tested against representative routes.
- [ ] Rate and cost model includes normal, peak, replay, and failure behavior.
- [ ] Retention and deletion requirements are recorded.
- [ ] Provider outage and termination fallback are documented.
- [ ] OD-002 is resolved in `../DECISIONS.md`.

Verification evidence: Pending.

## FT-302 — Integrate licensed live flight data

Status: Not started

Branch: `feat/ft-302-live-flight-integration`
Final commit: Pending
Pull request: Pending

Implement the selected provider adapter behind the canonical event boundary.

Dependencies: FT-301, FT-002, FT-104

Acceptance checklist:

- [ ] Provider adapter does not leak provider-specific types into the domain or UI.
- [ ] Schedule, identity, route, position, and status updates reconcile predictably.
- [ ] Rate limits, backfill, reconnect, and out-of-order delivery are tested.
- [ ] Data freshness and coverage quality are visible per flight.
- [ ] Replay fixtures can be produced lawfully without retaining prohibited fields.

Verification evidence: Pending.

## FT-303 — Add identity, roles, and tenant isolation

Status: Not started

Branch: `feat/ft-303-identity-tenant-isolation`
Final commit: Pending
Pull request: Pending

Protect real operational data and actions with authenticated, scoped access.

Dependencies: FT-001, FT-002

Acceptance checklist:

- [ ] Operator, dispatcher, viewer, and administrator permissions are documented.
- [ ] Every operational query and mutation is tenant-scoped.
- [ ] Cross-tenant access tests fail closed.
- [ ] Audit events include authenticated actor and tenant.
- [ ] Session expiry and revoked access produce safe UI behavior.
- [ ] OD-004 is resolved in `../DECISIONS.md`.

Verification evidence: Pending.

## FT-304 — Build the dispatcher review queue

Status: Not started

Branch: `feat/ft-304-dispatcher-review-queue`
Final commit: Pending
Pull request: Pending

Make alert triage fast, prioritized, and auditable for an operations user.

Dependencies: FT-204, FT-303

Acceptance checklist:

- [ ] Queue supports severity, status, flight, time, and assigned-user filters.
- [ ] Evidence is visible before an action is taken.
- [ ] Acknowledge, assign, dismiss, comment, and resolve actions have clear feedback.
- [ ] Concurrent updates do not silently overwrite another dispatcher’s action.
- [ ] Dismissal reasons are structured enough to tune alert rules.
- [ ] Queue usability is tested with a representative alert volume.

Verification evidence: Pending.
