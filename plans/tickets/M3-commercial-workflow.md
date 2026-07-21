# M3 — Commercial Data and Operational Workflow

Default owner: Product for provider contracting; engineering and security for implementation.

## FT-301 — Select and contract a commercial flight-data provider

Status: In progress

Branch: `docs/ft-301-commercial-provider-selection`
Final commit: Pending
Pull request: Pending
Owner: Product, legal, and engineering

Resolve the provider feasibility work with a documented selection appropriate for the intended use.

Dependencies: FT-003

Acceptance checklist:

- [ ] Commercial situational-display, tenant/customer, redistribution, source-combination, LADD/PIA, storage, and derived-data rights are confirmed in writing.
- [ ] Cirium Sky Stream and FlightAware Firehose run the same target-tail trial for at least 14 days; coverage and p50/p95/p99 latency are recorded by region.
- [ ] Rate and priced cost model covers 20, 100, and 500 monitored flights under normal, peak, replay, reconnect, and failure behavior.
- [ ] Retention and deletion requirements are recorded.
- [ ] Uptime definition, service credits, incident notification, support response, provider outage, and termination fallback are documented.
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

Status: Complete

Branch: `feat/ft-303-identity-tenant-isolation`
Final commit: `1430ce8`
Pull request: [#13](https://github.com/carlwelchdesign/flight-tracker-ai/pull/13)
Owner: Security and full-stack engineering

Protect real operational data and actions with authenticated, scoped access.

Dependencies: FT-001, FT-002

Acceptance checklist:

- [x] Operator, dispatcher, viewer, and administrator permissions are documented.
- [x] Every operational query and mutation is tenant-scoped.
- [x] Cross-tenant access tests fail closed.
- [x] Audit events include authenticated actor and tenant.
- [x] Session expiry and revoked access produce safe UI behavior.
- [x] OD-004 is resolved in `../DECISIONS.md`.

Verification evidence: [identity and tenant isolation contract](../IDENTITY_TENANT_ISOLATION.md); provider-neutral 30-second Next.js-to-Rust assertions with a 60-second API maximum; Clerk Organization and signed development adapters; app-owned memberships, roles, revocations, and authorization audit migration; authenticated tenant context across fleet list/detail/timeline, SSE replay/live delivery, weather, source evidence/health, alerts/actions, metrics, and replay controls; browser payloads no longer carry operator or actor authority; cross-tenant in-memory fleet IDs, PostGIS route reads, source-record denial, membership, expiry, revocation, and actor/tenant audit coverage; 60 Rust library tests, 8 binary configuration tests, deterministic golden tests, strict Clippy and formatting; 22 web parser, authorization, interaction, and revoked-session tests plus lint, typecheck, production build, and dependency audit with 0 vulnerabilities; live authenticated replay alert creation and actor-derived acknowledgement plus schema/PostGIS contracts in [CI run 29814499315](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29814499315); final implementation fix `1430ce8`; PR [#13](https://github.com/carlwelchdesign/flight-tracker-ai/pull/13), with all required checks passing.

## FT-304 — Build the dispatcher review queue

Status: Complete

Branch: `feat/ft-304-dispatcher-review-queue`
Final commit: `11bdc0d`
Pull request: [#14](https://github.com/carlwelchdesign/flight-tracker-ai/pull/14)
Owner: Backend and full-stack engineering

Make alert triage fast, prioritized, and auditable for an operations user.

Dependencies: FT-204, FT-303

Acceptance checklist:

- [x] Queue supports severity, status, flight, time, and assigned-user filters.
- [x] Evidence is visible before an action is taken.
- [x] Acknowledge, assign, dismiss, comment, and resolve actions have clear feedback.
- [x] Concurrent updates do not silently overwrite another dispatcher’s action.
- [x] Dismissal reasons are structured enough to tune alert rules.
- [x] Queue usability is tested with a representative alert volume.

Verification evidence: tenant-scoped Rust queue filters for severity, lifecycle, callsign or flight UUID, inclusive event time, and active assignee or unassigned state; active alert-manager directory and assignment audit records; atomic workflow-version increments with `409 alert_conflict` recovery; duplicate, stale-data, incorrect-correlation, not-operationally-relevant, and other dismissal taxonomy; evidence-first responsive interface with loading, filtered-empty, unavailable, partial assignee failure, read-only, pending, success, validation, and conflict states; PostGIS coverage for cross-tenant assignment denial, full filters, stale-write rejection, structured dismissal, and a bounded 100-of-120 alert page; 150-alert React usability coverage; 62 Rust library tests, 8 binary tests, strict Clippy, formatting, release build, 28 web tests, lint, typecheck, production build, and 0 dependency vulnerabilities; authenticated replay and migration/schema verification in [CI run 29816346733](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29816346733); implementation and CI contract commit `11bdc0d`; PR [#14](https://github.com/carlwelchdesign/flight-tracker-ai/pull/14), with all required checks passing.
