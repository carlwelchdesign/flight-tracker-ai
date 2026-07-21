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

- Next.js requires a hosted user, session, and active organization; the browser cannot submit operator authority. Its 30-second internal assertion names the active signing key with a safe `kid`.
- Rust validates the named active key and at most one named previous key during rotation, plus algorithm, signature, issuer, audience, required claims, not-before/expiry, and a maximum 60-second assertion lifetime, then resolves app-owned membership and revocation on every request.
- Production configuration rejects development authentication and development replay controls.
- Only minimal `/health` and `/readiness` status probes are unauthenticated; detailed service, worker, database, and PostGIS diagnostics live under authenticated `/api/system/*` routes. Operational reads, raw NOAA evidence, SSE, metrics, replay controls, alert actions, and membership administration also require authorization.
- Roles ask for named permissions. Viewers cannot mutate alerts; only administrators manage memberships/sessions or review/export tenant audit evidence.
- Store reads and writes take `operator_id` from `AuthContext`; assignment additionally verifies an active, enabled, same-operator dispatcher/operator/administrator, and composite database foreign keys prevent cross-operator alert and assignment-audit references.
- Alert actions use idempotency keys, row/advisory locks, expected workflow versions, structured dismissal reasons, and an append-only action history.
- Administrator audit review joins authorization and alert-action evidence under the authenticated tenant, bounds review/export windows and counts, excludes free-form comments/reasons, idempotency keys, and raw session IDs, and flags high-risk actions plus 15-minute actor bursts.
- Raw-provider-payload, normalized operational-fact, authorization-audit, expired session-revocation, exclusive inactive identity, and terminal alert-history retention use versioned tenant/provider policy, separate policy/run or standing schedule approval, fixed preview counts, locked recount, a 10,000-record safety bound, completion audit, supervised scheduled execution, durable failure attempts, restore-suppression tombstones, and administrator-only tenant integrity diagnostics. Shared identities, FT-301 contract values, managed execution, and the managed restore drill remain open.
- Raw source evidence is currently exposed only for tenant-scoped `noaa-awc` envelopes. A generic commercial raw-message route does not exist.
- Logs record method, route, status, latency, worker/source state, and safe correlation IDs rather than request bodies or bearer assertions.

## Threat model

| ID | Threat and abuse case | Existing control | Required treatment before pilot | Finding |
| --- | --- | --- | --- | --- |
| T-01 | Internal assertion secret or hosted identity key is exposed and attackers mint sessions. | Server-only assertion creation, minimum secret length, named active/previous keys, issuer/audience binding, short lifetime, forced retirement tests, browser-asset secret scan, database authorization, and the rotation/revocation runbook. | Configure managed environment-separated secret stores and complete normal/emergency hosted drills for internal assertions, Clerk, database, and provider credentials. | F401-001 |
| T-02 | Browser changes a tenant, actor, role, assignee, or alert identifier to cross operators. | Tenant/actor derive from verified context; composite tenant keys and scoped queries; active same-tenant assignee validation; composite assignment foreign keys; direct-database and API cross-tenant tests. | Closed: migration `20260721000500` prevents alternate write paths from creating cross-tenant alert assignments or assignment audit rows. | F401-004 |
| T-03 | Stolen or revoked hosted session continues to access operational data. | Assertions live 30 seconds; hosted sessions and app revocations are checked; membership status/identity disable fail closed; SSE reconnect reauthenticates; expired revocations and exclusive inactive identities have approved lifecycle cleanup with restore suppression. | Approve the cleanup schedule and complete the hosted revocation/restore operations drill. | F401-001, F401-008 |
| T-04 | Malformed, oversized, duplicated, out-of-order, or adversarial provider messages exhaust workers or poison state. | NOAA timeouts/retry/cadence; provider-envelope dedupe; canonical validation; bounded channels; deterministic rule boundaries. | FT-302 adapter must impose size/rate/schema limits and quarantine with bounded evidence; FT-402 must drill poison-message and backlog recovery. | F401-006 |
| T-05 | Stale/partial data, green transport, or model wording is mistaken for current authoritative guidance. | Separate event/receive/process times, source health, stale/degraded UI, deterministic alerts, human actions, flight-detail copy, and a root-level advisory/source-authority banner with explicit simulation/evaluation modes. | Obtain Product/Legal/operator approval for the exact wording and review the real preview across empty/error/queue/detail states. | F401-009 |
| T-06 | Dispatcher or administrator performs a privileged action without accountability or overwrites another actor. | Named permissions, authenticated actor, idempotency, workflow-version conflict, structured reasons, alert/authorization audit, administrator-only tenant review, bounded redacted export, deterministic high-risk/burst signals, and approved authorization-audit deletion with restore suppression. | Complete the hosted retention/integrity and incident drill in `AUDIT_REVIEW_RUNBOOK.md`. | F401-007 |
| T-07 | Commercial data is displayed, retained, exported, combined, or processed by AI beyond contract scope. | FT-301 deny-by-default gate; commercial provider not integrated; raw route restricted to NOAA; provider/question evidence validator. | Implement selected Order's entitlement, attribution, field, retention, deletion, blocked-tail, export, and AI rules before adapter activation. | F401-003 |
| T-08 | Data survives revocation, termination, deletion request, backup expiry, or provider deadline. | No production commercial ingestion exists. Raw payload, normalized operational fact, authorization audit, expired session revocation, exclusive inactive identity, and terminal alert-history classes have two-person policy/run or standing schedule approval, preview/recount, deletion evidence, supervised execution/failure state, restore-suppression tombstones, a controlled restore runbook, and tenant-scoped resurrection checks. | Decide shared-identity disposition; apply FT-301 contract values; complete managed deletion and backup/tombstone restore drills. | F401-002, F401-008 |
| T-09 | Public probes or browser responses reveal unnecessary topology, worker, identity, or operational detail. | Public probes expose one status field; detailed diagnostics require app authorization; generic auth failures and BFF path/header allowlists prevent accidental detail forwarding; baseline browser headers and Clerk strict CSP are configured. | F401-010 is closed. Complete F401-005 with a real hosted-Clerk browser smoke before sharing a preview externally. | F401-005, F401-010 |
| T-10 | Free-form notes, support exports, or correlation IDs become a channel for secrets or personal data. | Correlation IDs allow a small safe character set; operations runbook forbids sensitive values; comments remain tenant-scoped; audit review/export excludes free-form comments/reasons, raw session IDs, and idempotency keys. | Add bounded write lengths, approved retention, and incident scanning before pilot. | F401-002, F401-007 |

## Advisory-language review

| Surface | Current evidence | Result |
| --- | --- | --- |
| Product/README metadata | Describes an advisory operations console and excludes autonomous dispatch, certified planning, pilot commands, and LLM safety decisions. | Pass for current scope. |
| Flight detail | States “Advisory display only” and requires source verification before operational action. | Pass. |
| Source/freshness states | Distinguishes current, stale, degraded, unavailable, event time, receipt time, and transport state. | Pass. |
| Alert queue | Uses “Decision support,” “Evidence before action,” versioned rule/score evidence, human notes, explicit action history, and inherits the persistent root-level limitation. | Pass for implemented copy. |
| Global environment banner | Always states the configured simulation/evaluation mode, advisory-only limitation, source scope, and source-authority/freshness verification instruction. Invalid modes fail closed. | Pass for implementation; exact pilot wording approval remains pending. |
| Empty, loading, signed-out, error states | Inherit the root-level environment/advisory banner and avoid operational recommendations. | Pass for implementation; real preview state review remains pending. |

Exact pilot wording requires Product, Legal, and the operator partner. Until F401-009 closes, the “advisory-only product language is reviewed and consistent” acceptance item remains open.

## Provider and attribution gate

Current NOAA evidence retains provider/feed identity, envelope ID, event/receive/process times, raw hash, and tenant-scoped raw public evidence. This does **not** authorize or implement the commercial provider behavior.

After FT-301 selects a provider, the adapter design must map every accepted/exception R/S response to one of: configuration constraint, field suppression, entitlement check, UI attribution, export restriction, retention/deletion rule, audit event, test, or explicit no-use. Legal must approve the mapping before credentials or payloads enter a shared environment. F401-003 cannot close from public terms or a green technical trial alone.

## Review traceability

| Review lens | Main conclusion | Canonical control |
| --- | --- | --- |
| Trust/privacy/rights | Identity controls are credible, but commercial rights, deletion, export, and AI policy remain contract-specific. | FT-301 matrix plus F401-002/F401-003/F401-007/F401-008. |
| Platform architecture | Tenant scope and assignment integrity are enforced, and public probes are minimal. Secret rotation, abuse quarantine, and hosted-Clerk CSP verification remain open. | F401-004/F401-010 closed; F401-001/F401-005/F401-006 remain open. |
| Delivery/TPM | No finding may disappear into prose; each has an owner, severity, deadline gate, status, and verification requirement. | `SECURITY_FINDINGS.csv` and FT-401 checklist. |

## Approval rule

FT-401 completes only when all `critical` and `high` findings are `closed`, any remaining `medium` finding has explicit Product/Security risk acceptance with an expiry, F401-003 is backed by the selected provider's controlling terms, and the ticket acceptance checklist cites verification evidence. A document-only review cannot authorize a pilot.
