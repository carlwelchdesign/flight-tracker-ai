# Security, Privacy, and Trust Review

FT-401 review baseline, last reviewed 2026-07-21. This document evaluates the implemented FT-303/FT-304 system and defines the controls required before any real-operations pilot. It is not a penetration test, legal approval, provider contract approval, or production authorization.

## Decision state

- Review status: **in progress**.
- Current environment authorized for: local deterministic simulation and licensed/public NOAA evaluation only.
- Real commercial flight data: **not authorized** until FT-301 resolves OD-002 and every provider-specific control below is implemented.
- Pilot approval: **not granted**. Open findings are authoritative in [`SECURITY_FINDINGS.csv`](SECURITY_FINDINGS.csv).

## Assets and data classes

| Class | Examples | Handling rule |
| --- | --- | --- |
| Public | NOAA observations/hazards, public product documentation | Preserve source attribution and timestamps; public origin does not make modified output official. |
| Internal operational | Normalized flights/routes/positions, alerts, source health, aggregate metrics | Tenant-scoped access; no public exports; source/freshness remains visible. |
| Restricted operational | Commercial raw messages, real tail population, dispatcher notes, audit history, blocked-aircraft entitlement | Minimum fields; server-side only; contract-specific retention/deletion; controlled exports; no AI use by default. |
| Restricted identity | External subject/tenant/session identifiers, roles, memberships, revocations | App-owned authorization; administrator-only management; do not log bearer assertions or session tokens. |
| Confidential commercial | Orders, licenses, quotes, SLA/security schedules, provider replies | Controlled contract/procurement systems only; Git stores opaque references and redacted summaries. |
| Secret | Clerk keys, internal assertion secret, database/provider credentials | Managed secret store; environment separation; rotation and incident revocation; never browser-visible or committed. |

Passenger, crew, payment, passport, medical, and communications content are outside the approved MVP. Adding any of them requires a new data-protection review and ticket.

## Implemented trust boundaries

```text
browser -> verified Clerk session -> Next.js server-only BFF
        -> 30-second HS256 internal assertion -> Rust verification
        -> database membership/session revocation -> AuthContext
        -> permission check -> tenant-scoped store/query -> PostgreSQL/PostGIS

public/licensed source -> bounded adapter -> immutable provider envelope
                       -> normalization -> tenant-scoped facts/rules
                       -> source/freshness evidence -> human review/action audit
```

Evidence observed in the repository:

- Next.js requires a hosted user, session, and active organization; the browser cannot submit operator authority.
- Rust validates algorithm, signature, issuer, audience, required claims, not-before/expiry, and a maximum 60-second assertion lifetime, then resolves app-owned membership and revocation on every request.
- Production configuration rejects development authentication and development replay controls.
- Only `/health` and `/readiness` are unauthenticated; operational reads, raw NOAA evidence, SSE, metrics, replay controls, alert actions, and membership administration require authorization.
- Roles ask for named permissions. Viewers cannot mutate alerts; only administrators manage memberships/sessions.
- Store reads and writes take `operator_id` from `AuthContext`; assignment additionally verifies an active, enabled, same-operator dispatcher/operator/administrator.
- Alert actions use idempotency keys, row/advisory locks, expected workflow versions, structured dismissal reasons, and an append-only action history.
- Raw source evidence is currently exposed only for tenant-scoped `noaa-awc` envelopes. A generic commercial raw-message route does not exist.
- Logs record method, route, status, latency, worker/source state, and safe correlation IDs rather than request bodies or bearer assertions.

## Threat model

| ID | Threat and abuse case | Existing control | Required treatment before pilot | Finding |
| --- | --- | --- | --- | --- |
| T-01 | Internal assertion secret or hosted identity key is exposed and attackers mint sessions. | Server-only assertion creation, minimum secret length, issuer/audience binding, short lifetime, database authorization. | Managed secret storage, environment isolation, dual-key/rotation procedure, emergency revocation, and proof that secrets never reach browser/build output. | F401-001 |
| T-02 | Browser changes a tenant, actor, role, assignee, or alert identifier to cross operators. | Tenant/actor derive from verified context; composite tenant keys and scoped queries; same-tenant assignee validation; cross-tenant tests. | Add database-level operator/assignee integrity so alternate write paths cannot create cross-tenant assignments. | F401-004 |
| T-03 | Stolen or revoked hosted session continues to access operational data. | Assertions live 30 seconds; hosted sessions and app revocations are checked; membership status/identity disable fail closed; SSE reconnect reauthenticates. | Define identity-disable/revocation operations runbook and maximum revocation-record cleanup delay. | F401-001, F401-008 |
| T-04 | Malformed, oversized, duplicated, out-of-order, or adversarial provider messages exhaust workers or poison state. | NOAA timeouts/retry/cadence; provider-envelope dedupe; canonical validation; bounded channels; deterministic rule boundaries. | FT-302 adapter must impose size/rate/schema limits and quarantine with bounded evidence; FT-402 must drill poison-message and backlog recovery. | F401-006 |
| T-05 | Stale/partial data, green transport, or model wording is mistaken for current authoritative guidance. | Separate event/receive/process times, source health, stale/degraded UI, deterministic alerts, human actions, advisory copy in flight detail. | Persistent pilot-wide advisory/environment label, approved exact wording, and tests across empty/error/queue/detail states. | F401-009 |
| T-06 | Dispatcher or administrator performs a privileged action without accountability or overwrites another actor. | Named permissions, authenticated actor, idempotency, workflow-version conflict, structured reasons, alert/authorization audit. | Define audit access/export, retention, integrity review, and privileged-action alerting. | F401-007 |
| T-07 | Commercial data is displayed, retained, exported, combined, or processed by AI beyond contract scope. | FT-301 deny-by-default gate; commercial provider not integrated; raw route restricted to NOAA; provider/question evidence validator. | Implement selected Order's entitlement, attribution, field, retention, deletion, blocked-tail, export, and AI rules before adapter activation. | F401-003 |
| T-08 | Data survives revocation, termination, deletion request, backup expiry, or provider deadline. | No production commercial ingestion exists. | Implement object inventory, scheduled deletion, backup expiry, restore-time tombstones, deletion evidence, and contract-specific overrides. | F401-002, F401-008 |
| T-09 | Public probes or browser responses reveal unnecessary topology, worker, identity, or operational detail. | Generic auth failures; no stack traces/request bodies returned; BFF allowlists paths/headers. | Minimize externally exposed probe payloads and add production response security headers. | F401-005, F401-010 |
| T-10 | Free-form notes, support exports, or correlation IDs become a channel for secrets or personal data. | Correlation IDs allow a small safe character set; operations runbook forbids sensitive values; comments remain tenant-scoped. | Add user guidance, export redaction, bounded lengths, retention, and incident scanning before pilot. | F401-002, F401-007 |

## Advisory-language review

| Surface | Current evidence | Result |
| --- | --- | --- |
| Product/README metadata | Describes an advisory operations console and excludes autonomous dispatch, certified planning, pilot commands, and LLM safety decisions. | Pass for current scope. |
| Flight detail | States “Advisory display only” and requires source verification before operational action. | Pass. |
| Source/freshness states | Distinguishes current, stale, degraded, unavailable, event time, receipt time, and transport state. | Pass. |
| Alert queue | Uses “Decision support,” “Evidence before action,” versioned rule/score evidence, human notes, and explicit action history. | Partial: no persistent advisory limitation within the queue. |
| Global footer/environment | States “Simulation environment” and “human-reviewed decisions.” | Pass for development; unsafe if reused unchanged for live pilot data. |
| Empty, loading, signed-out, error states | Avoid operational recommendations and preserve/de-identify data appropriately. | Pass, subject to persistent pilot label. |

Exact pilot wording requires Product, Legal, and the operator partner. Until F401-009 closes, the “advisory-only product language is reviewed and consistent” acceptance item remains open.

## Provider and attribution gate

Current NOAA evidence retains provider/feed identity, envelope ID, event/receive/process times, raw hash, and tenant-scoped raw public evidence. This does **not** authorize or implement the commercial provider behavior.

After FT-301 selects a provider, the adapter design must map every accepted/exception R/S response to one of: configuration constraint, field suppression, entitlement check, UI attribution, export restriction, retention/deletion rule, audit event, test, or explicit no-use. Legal must approve the mapping before credentials or payloads enter a shared environment. F401-003 cannot close from public terms or a green technical trial alone.

## Review traceability

| Review lens | Main conclusion | Canonical control |
| --- | --- | --- |
| Trust/privacy/rights | Identity controls are credible, but commercial rights, deletion, export, and AI policy remain contract-specific. | FT-301 matrix plus F401-002/F401-003/F401-007/F401-008. |
| Platform architecture | Tenant scope is enforced in application queries, while secret rotation, DB-level assignment integrity, abuse quarantine, and probe/header hardening remain open. | F401-001/F401-004/F401-005/F401-006/F401-010. |
| Delivery/TPM | No finding may disappear into prose; each has an owner, severity, deadline gate, status, and verification requirement. | `SECURITY_FINDINGS.csv` and FT-401 checklist. |

## Approval rule

FT-401 completes only when all `critical` and `high` findings are `closed`, any remaining `medium` finding has explicit Product/Security risk acceptance with an expiry, F401-003 is backed by the selected provider's controlling terms, and the ticket acceptance checklist cites verification evidence. A document-only review cannot authorize a pilot.
