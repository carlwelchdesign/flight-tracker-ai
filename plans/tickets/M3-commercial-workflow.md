# M3 — Portfolio Live Data and Operational Workflow

Default owner: Product and engineering for source eligibility; engineering and security for implementation.

## FT-301 — Select an eligible free aircraft-position source

Status: Complete

Branch: `docs/ft-301-free-data-selection`
Final commit: `13d64eb`
Pull request: [#20](https://github.com/carlwelchdesign/flight-tracker-ai/pull/20)
Owner: Product and engineering

Select a zero-data-fee, best-effort aircraft-position source whose official terms permit a publicly hosted, non-commercial portfolio demonstration. Selecting replay-only is an acceptable outcome if no candidate has sufficiently clear terms.

Dependencies: FT-003

Acceptance checklist:

- [x] Official terms are linked and permit server-side access plus public, hosted, non-commercial display for this portfolio project.
- [x] Attribution, caching, retention, redistribution, rate-limit, and acceptable-use requirements are recorded.
- [x] A bounded sample documents coverage gaps, missing fields, freshness, and failure behavior without treating best-effort data as complete.
- [x] The selected source supplies positions only unless its documented schema proves additional facts; scenario routes, schedules, and statuses remain visibly simulated.
- [x] Replay remains the default fallback when the source is unavailable, rate-limited, ineligible, or too incomplete for a convincing demo.
- [x] No SLA, procurement process, paid trial, legal department, operator partner, or commercial-use approval is required for the portfolio release.
- [x] OD-002 is resolved in `../DECISIONS.md` with a selected source or an explicit replay-only decision.

Verification evidence: [`ADSBLOL_INTEGRATION.md`](../ADSBLOL_INTEGRATION.md) selects
ADSB.lol for an optional ephemeral position layer, records ODbL attribution and
`no-store` controls, captures a three-region bounded sample, and preserves
deterministic replay as the only fallback. ADR-011 resolves OD-002. The archived
commercial package and FT-401 review remain valid, all five FT-301 validator
tests pass, and Rust, web, and API/PostGIS checks pass in CI run
[29852787739](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29852787739).

Historical evidence: PR [#15](https://github.com/carlwelchdesign/flight-tracker-ai/pull/15) and PR [#16](https://github.com/carlwelchdesign/flight-tracker-ai/pull/16) produced a complete commercial procurement framework. It is retained under [`provider-evaluation/`](../provider-evaluation/README.md) for a possible future production track but is not part of this ticket's acceptance gate.

## FT-302 — Integrate best-effort live aircraft positions

Status: Complete

Branch: `feat/ft-302-live-flight-integration`
Final implementation commit: `fe8957b`
Pull request: [#21](https://github.com/carlwelchdesign/flight-tracker-ai/pull/21)
Owner: Backend and full-stack engineering

Implement the selected free source behind the canonical event boundary without weakening deterministic replay.

Dependencies: FT-301, FT-002, FT-104

Acceptance checklist:

- [x] Provider adapter does not leak provider-specific types into the domain or UI.
- [x] Position and available identity fields reconcile predictably; unsupported schedule, route, and status facts are not invented.
- [x] Rate limits, timeouts, reconnect, duplicate, stale, and out-of-order delivery are tested.
- [x] Source, attribution, freshness, coverage quality, and best-effort status are visible per flight.
- [x] The UI persistently states `Portfolio demonstration — not for operational use` for both live and replay modes.
- [x] Feed failure automatically preserves a usable replay path and visibly reports the source as unavailable or degraded.
- [x] The ADSB.lol adapter uses only a bounded regional endpoint, polls no faster than every 30 seconds, permits one request in flight, times out, backs off with jitter, and never performs global or per-aircraft polling.
- [x] ADSB.lol responses remain ephemeral and uncached across Rust and Next.js, preserve `Cache-Control: no-store`, are excluded from logs, analytics, exports, LLM inputs, fixtures, PostgreSQL, and backups, and receive linked ODbL attribution whenever visible.
- [x] Stored samples and fixtures contain replay-owned synthetic data only; no ADSB.lol response body is committed or retained.

Verification evidence: the provider-private Rust adapter maps only allowlisted
identity, position, motion, and source-quality facts into canonical events; a
sequential runtime enforces one bounded regional request, a 30-second minimum
cadence, five-second timeout, one-megabyte response cap, and jittered bounded
retry. The live batch reaches only the in-memory fleet projection. The
tenant-scoped source-status API, fleet reads, and Next.js proxy preserve
`Cache-Control: no-store`. The console distinguishes live facts from simulated
route/schedule/status, shows per-flight source quality and freshness, includes
linked ODbL attribution, and preserves an explicit replay action during
degraded/unavailable states. Repository verification passes 84 Rust library
tests, 11 binary configuration tests, 4 integration/schema tests, strict
Clippy, formatting, 50 web tests, lint, typecheck, and a production Next.js
build. API/PostGIS verification remained assigned to CI because the local
PostgreSQL 14 service does not have PostGIS installed. CI run
[29855008220](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29855008220)
passes Rust, web, and API/PostGIS checks, including authenticated
disabled-by-default source state and end-to-end fleet/status `no-store`
headers.

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
