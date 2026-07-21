# Security, Privacy, and Trust Review

FT-401 review baseline, last reviewed 2026-07-21. This document evaluates the implemented FT-303/FT-304 system for a public, non-commercial portfolio demonstration. It is not a penetration test, safety certification, operational approval, or authorization to publish an unverified hosted environment.

## Decision state

- Review status: **approved for repository scope**.
- Current environment authorized for: local deterministic replay and public NOAA evaluation.
- Public deployment: gated by FT-404 environment configuration and smoke evidence; this review does not claim that a hosted environment already exists.
- External aircraft positions: replay-only is valid. FT-301/FT-302 must verify and implement an eligible free source's terms before activation.
- Residual findings are explicitly controlled and transferred to FT-302, FT-402, or FT-404 in [`SECURITY_FINDINGS.csv`](SECURITY_FINDINGS.csv).

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
- Alert actions use normalized bounded idempotency keys, bounded actor identifiers and dispatcher notes, row/advisory locks, expected workflow versions, structured dismissal reasons, and an append-only action history. Session-revocation provider/session/reason fields are bounded in Rust and PostgreSQL.
- Administrator audit review joins authorization and alert-action evidence under the authenticated tenant, bounds review/export windows and counts, excludes free-form comments/reasons, idempotency keys, and raw session IDs, and flags high-risk actions plus 15-minute actor bursts. A separate bounded scan inspects dispatcher comments and revocation reasons for credential material or personal email addresses, but emits only field class, severity, actor, time, and record ID.
- Raw-provider-payload, normalized operational-fact, authorization-audit, expired session-revocation, exclusive inactive identity, and terminal alert-history retention use versioned tenant/provider policy, separate policy/run or standing schedule approval, fixed preview counts plus an exact-key SHA-256 fingerprint, repeatable-read execution, locked recount, a 10,000-record safety bound, completion audit, supervised scheduled execution, durable failure attempts, restore-suppression tombstones, and administrator-only tenant integrity diagnostics. Environment-specific scheduling and restore evidence belong to FT-404/FT-402.
- Raw source evidence is currently exposed only for tenant-scoped `noaa-awc` envelopes. A generic commercial raw-message route does not exist.
- Logs record method, route, status, latency, worker/source state, and safe correlation IDs rather than request bodies or bearer assertions.

## Threat model

| ID | Threat and abuse case | Existing control | Downstream activation or deployment gate | Finding |
| --- | --- | --- | --- | --- |
| T-01 | Internal assertion secret or hosted identity key is exposed and attackers mint sessions. | Server-only assertion creation, minimum secret length, named active/previous keys, issuer/audience binding, short lifetime, forced retirement tests, browser-asset secret scan, database authorization, and the rotation/revocation runbook. | FT-404 configures environment-separated secrets and runs the hosted smoke before a public URL is approved. | F401-001 |
| T-02 | Browser changes a tenant, actor, role, assignee, or alert identifier to cross operators. | Tenant/actor derive from verified context; composite tenant keys and scoped queries; active same-tenant assignee validation; composite assignment foreign keys; direct-database and API cross-tenant tests. | Closed: migration `20260721000500` prevents alternate write paths from creating cross-tenant alert assignments or assignment audit rows. | F401-004 |
| T-03 | Stolen or revoked hosted session continues to access portfolio data. | Assertions live 30 seconds; hosted sessions and app revocations are checked; membership status/identity disable fail closed; SSE reconnect reauthenticates; expired revocations and exclusive inactive identities have approved lifecycle cleanup with restore suppression. | FT-404 verifies the chosen identity deployment; FT-402 verifies recovery when persistent hosting is enabled. | F401-001, F401-008 |
| T-04 | Malformed, oversized, duplicated, out-of-order, or adversarial provider messages exhaust workers or poison state. | NOAA timeouts/retry/cadence; provider-envelope dedupe; canonical validation; bounded channels; deterministic rule boundaries. | FT-302 adapter must impose size/rate/schema limits and quarantine with bounded evidence; FT-402 must drill poison-message and backlog recovery. | F401-006 |
| T-05 | Stale/partial data, green transport, or model wording is mistaken for current authoritative guidance. | Separate event/receive/process times, source health, stale/degraded UI, deterministic alerts, human actions, and a global portfolio/not-for-operational-use banner with explicit simulation/evaluation modes. | F401-009 is closed for repository scope; FT-404 visually verifies the same global copy in the deployed URL. | F401-009 |
| T-06 | A portfolio user or administrator performs a privileged action without accountability or overwrites another actor. | Named permissions, authenticated actor, idempotency, workflow-version conflict, structured reasons, alert/authorization audit, administrator-only tenant review, bounded redacted export, deterministic high-risk/burst and sensitive-write signals, approved authorization-audit deletion with restore suppression, and a sanitized fail-closed hosted verifier. | FT-404 runs the verifier against the chosen environment; FT-402 owns incident exercises. | F401-007 |
| T-07 | External flight data is displayed or retained beyond the source's terms. | No commercial or free aircraft-position adapter is active; raw source evidence is restricted to NOAA; replay remains the default. | F401-003 is closed for current scope. FT-301/FT-302 must implement official terms, attribution, retention, rate limits, and acceptable use before any free feed is enabled. | F401-003 |
| T-08 | Portfolio data survives revocation, deletion, or backup expiry. | Raw payload, normalized operational fact, authorization audit, expired session revocation, exclusive inactive identity, and terminal alert-history classes have two-person policy/run or standing schedule approval, preview/recount, deletion evidence, supervised execution/failure state, restore-suppression tombstones, a controlled restore runbook, and tenant-scoped resurrection checks. | FT-404 configures environment-specific schedules and backup policy; FT-402 records recovery evidence for persistent hosting. | F401-002, F401-008 |
| T-09 | Public probes or browser responses reveal unnecessary topology, worker, identity, or operational detail. | Public probes expose one status field; detailed diagnostics require app authorization; generic auth failures and BFF path/header allowlists prevent accidental detail forwarding; baseline browser headers and Clerk strict CSP are configured. | F401-010 is closed. FT-404 completes the real hosted-Clerk browser smoke before publication. | F401-005, F401-010 |
| T-10 | Free-form notes, support exports, or correlation IDs become a channel for secrets or personal data. | Correlation IDs allow a small safe character set; operations runbook forbids sensitive values; alert notes, action identifiers, and revocation reasons have Rust/database length limits; comments remain tenant-scoped and follow approved terminal-alert retention; audit review/export excludes free-form comments/reasons, raw session IDs, and idempotency keys; a deterministic bounded scan detects credential material and personal email addresses without returning matched content. | FT-404 executes the bounded hosted verifier before publication. | F401-007 |

## Advisory-language review

| Surface | Current evidence | Result |
| --- | --- | --- |
| Product/README metadata | Identifies a non-commercial portfolio demonstration for recruiters and hiring managers and excludes operational use, autonomous dispatch, certified planning, pilot commands, and LLM safety decisions. | Pass. |
| Flight detail | States “Portfolio demonstration only,” prohibits operational use, and retains source-verification guidance. | Pass. |
| Source/freshness states | Distinguishes current, stale, degraded, unavailable, event time, receipt time, and transport state. | Pass. |
| Alert queue | Uses “Decision support,” “Evidence before action,” versioned rule/score evidence, human notes, explicit action history, and inherits the persistent root-level limitation. | Pass for implemented copy. |
| Global environment banner | Always states the configured simulation/evaluation mode, portfolio limitation, prohibition on operational use, source scope, and source-authority/freshness verification instruction. Invalid modes fail closed. | Pass. |
| Empty, loading, signed-out, error states | Inherit the root-level portfolio banner and avoid operational recommendations. | Pass; FT-404 retains deployed visual smoke. |

ADR-010 is the project-owner wording decision. F401-009 closes because the global UI and metadata implement that decision and tests cover both supported modes. FT-404 verifies rendering in the real deployment without reopening a commercial or operator-approval gate.

## External source activation gate

Current NOAA evidence retains provider/feed identity, envelope ID, event/receive/process times, raw hash, and tenant-scoped raw public evidence. No aircraft-position source other than replay is enabled.

If FT-301 selects a free best-effort source, FT-302 must map its official terms to configuration constraints, field suppression, UI attribution, caching and retention rules, rate limits, acceptable-use controls, tests, or explicit no-use before credentials or payloads enter a hosted environment. Replay-only is a valid outcome. Commercial procurement remains an optional future track and is not an FT-401 gate.

## Review traceability

| Review lens | Main conclusion | Canonical control |
| --- | --- | --- |
| Trust/privacy/rights | Current sources and replay are bounded; any future free feed has an explicit activation gate. Retention and audit controls are implemented at repository level. | FT-301/FT-302 plus controlled F401-002/F401-003/F401-007/F401-008. |
| Platform architecture | Tenant scope, assignment integrity, minimal public probes, rotation mechanics, CSP, audit, retention, and recovery checks are implemented. Environment-specific evidence belongs to FT-402/FT-404. | F401-004/F401-009/F401-010 closed; F401-001/F401-005/F401-007/F401-008 controlled by downstream gates. |
| Delivery/TPM | No residual risk disappears into prose; each controlled item has an owner, downstream ticket, expiry, and verification evidence. | `SECURITY_FINDINGS.csv`, FT-302, FT-402, FT-404, and the FT-401 checklist. |

## Approval rule

FT-401 completes when all `critical` and `high` findings are closed, each residual `medium` finding is controlled by an explicit downstream activation/deployment ticket and expiring project-owner risk acceptance, the repository checks pass, and the ticket cites verification evidence. Completion approves the repository security baseline only. FT-302 authorizes a future free-feed adapter, FT-402 verifies failure/recovery behavior, and FT-404 authorizes a public hosted deployment.
