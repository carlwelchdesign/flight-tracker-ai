# M4 — Portfolio Launch and Demonstration Hardening

Default owner: Product and engineering, supported by security.

## FT-401 — Complete security, privacy, and trust review

Status: In progress

Branch: `docs/ft-401-security-trust-review`
Latest implementation commit: `4649408`
Final commit: Pending
Pull request: [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18) (draft; main merged and conflicts resolved)
Owner: Product, engineering, and security
Completion boundary: FT-401 approves the repository security baseline. FT-302 owns any future free-feed activation controls, FT-402 owns failure/recovery exercises, and FT-404 owns hosted secrets, identity, backups, browser smoke, and public-deployment approval. Commercial-provider procurement and real-operator approval are not dependencies. A replay-only deployment remains valid.

Review data handling, permissions, auditability, portfolio language, and source obligations before making the demonstration public.

Dependencies: FT-303, FT-304

Preparation scope: review the implemented FT-303/FT-304 boundaries for a public portfolio. Any future free-feed attribution, retention, and acceptable-use controls belong to FT-301/FT-302 and apply only when that feed is enabled.

Preparation checklist:

- [x] Audit implemented authentication, tenancy, ingestion, audit, privileged-action, and advisory-language boundaries.
- [x] Create the threat model and proposed data lifecycle, backup, and incident-response baseline.
- [x] Record severity, owner, deadline gate, treatment, and verification for every finding.
- [x] Bound and scan sensitive user-written operational fields without returning their content.
- [x] Provide a fail-closed hosted audit/retention drill verifier with sanitized evidence output.
- [x] Remove commercial-provider procurement and real-operator approval from the portfolio completion gate while preserving an explicit activation gate for any future free feed.

Acceptance checklist:

- [x] Threat model covers credentials, tenant isolation, ingestion abuse, and privileged actions.
- [x] Retention, deletion, backup, and incident-response policies are documented.
- [x] Portfolio-only and not-for-operational-use language is consistent across normal, loading, empty, degraded, and error states.
- [x] No unsupported external flight-position source is enabled; future free-feed attribution, rate-limit, caching, and retention obligations remain an FT-301/FT-302 activation gate, and replay-only deployment is valid.
- [x] Security findings have owners and deadlines.

Verification evidence: [`SECURITY_PRIVACY_TRUST_REVIEW.md`](../SECURITY_PRIVACY_TRUST_REVIEW.md), [`DATA_LIFECYCLE_INCIDENT_POLICY.md`](../DATA_LIFECYCLE_INCIDENT_POLICY.md), [`CREDENTIAL_ROTATION_RUNBOOK.md`](../CREDENTIAL_ROTATION_RUNBOOK.md), [`AUDIT_REVIEW_RUNBOOK.md`](../AUDIT_REVIEW_RUNBOOK.md), [`RETENTION_DELETION_RUNBOOK.md`](../RETENTION_DELETION_RUNBOOK.md), [`BACKUP_RESTORE_RUNBOOK.md`](../BACKUP_RESTORE_RUNBOOK.md), [`SECURITY_FINDINGS.csv`](../SECURITY_FINDINGS.csv), and `python3 scripts/validate_ft401_review.py --require-complete`. Critical/high findings are closed; residual medium findings have explicit downstream FT-302/FT-402/FT-404 gates and expiring portfolio risk acceptance. CI run [29833385671](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29833385671) closes F401-004 through operator-scoped assignment constraints. CI run [29834083229](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834083229) closes F401-010 through exact minimal public probes and authenticated diagnostics against PostGIS. CI run [29834813131](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834813131) verifies the named assertion rotation protocol and browser-asset secret scan; F401-001 is controlled by FT-404 before publication. Commit `54db2b1` adds a root-level, mode-aware source-authority banner; the FT-401 closeout updates it, page metadata, and flight detail to say `Portfolio demonstration — not for operational use`. Commit `b022142` adds administrator-only tenant audit review, bounded redacted CSV export, and deterministic high-risk/burst signals; CI run [29836717456](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29836717456) passes Rust, web, and PostGIS contracts. Commits `f5ba6c1`, `fdf75af`, `b937f8a`, `420f145`, `5c320b8`, `ec9ea2a`, `a521e91`, and `cd08649` implement and verify approval-based retention, bounded scheduling, exact inventory, tombstones, and tenant resurrection diagnostics. F401-002, F401-005, F401-007, and F401-008 are controlled by explicit hosting/recovery gates rather than falsely reported as hosted evidence.

Exact-inventory evidence: commit `a521e91` binds every new retention run to a SHA-256 fingerprint of its eligible record keys under repeatable-read execution. CI run [29843308802](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29843308802) proves a same-count record substitution is rejected without deletion and that restoring the original inventory reproduces its fingerprint.

Bounded-write evidence: commit `cd08649` applies matching Rust and PostgreSQL limits to dispatcher notes, action identifiers, and session-revocation reasons, normalizes idempotency before locking and lookup, returns typed 422 errors, and exposes the 2,000-character note limit in the console. CI run [29844162906](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29844162906) proves API rejection, direct-database enforcement, 71 Rust library tests, 43 web tests, lint, typecheck, production build, and the PostGIS contract.

Sensitive-write monitoring evidence: commit `d975ac3` adds a deterministic Rust policy that scans bounded dispatcher comments and session-revocation reasons for credential material or personal email addresses inside the administrator-only tenant boundary. Signals disclose only severity, actor, time, record ID, and field class. CI run [29845085036](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29845085036) proves controlled credential/email detection, ordinary aviation-text rejection, response non-leakage, and cross-tenant isolation. F401-007 is controlled by the FT-404 hosted verification gate.

Hosted-verifier evidence: commits `943bb65` and `f3d3e08` add a bounded HTTPS verifier for administrator audit/export/monitoring/integrity access, viewer/operator denial, expected sensitive-write records, cross-tenant exclusion, controlled-marker redaction, and exact retention disposition counts. Its sanitized evidence allowlists output fields and never includes tokens, response bodies, markers, or event IDs. CI run [29846123252](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29846123252) passes the ten-case regression suite plus all Rust, web, and PostGIS jobs. FT-404 must run this already-tested verifier against the chosen hosted environment before publication.

Scope-reconciliation evidence (2026-07-21): FT-005 and ADR-010 remove commercial contract evidence, paid trials, pricing, SLA, and real-operator approval from this ticket. The implemented controls remain intact. `python3 scripts/validate_ft401_review.py --require-complete` passes after every residual risk is closed or controlled by a named downstream ticket and expiry; `validate_ft301_evidence.py --require-complete` is not an FT-401 completion condition. Required PR CI remains pending for the final closeout commit.

## FT-402 — Run resilience and failure drills

Status: Not started

Branch: `feat/ft-402-resilience-drills`
Final commit: Pending
Pull request: Pending

Demonstrate a reliable recruiter-facing experience under feed, database, worker, network, and malformed-data failures.

Dependencies: FT-302, FT-304

Acceptance checklist:

- [ ] Free-feed outage and high-latency checks produce visible degraded states and leave replay available.
- [ ] Worker restart does not duplicate or lose lifecycle history beyond documented guarantees.
- [ ] Database recovery procedure is tested.
- [ ] Malformed and adversarial provider payloads are rejected or quarantined.
- [ ] Alert backlog recovery behavior is measured.
- [ ] Demo and developer runbooks are updated from the findings.

Verification evidence: Pending.

## FT-403 — Validate the recruiter and hiring-manager demo

Status: Not started

Branch: `docs/ft-403-portfolio-demo-validation`
Final commit: Pending
Pull request: Pending

Run a focused usability evaluation with representative recruiters, hiring managers, or neutral reviewers and explicit success/failure criteria.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [ ] Demo scope, viewers, tasks, data modes, and prohibited operational uses are written.
- [ ] Measures include time to understand the product, task completion, source-mode comprehension, and data availability.
- [ ] Users complete core workflows without facilitator intervention.
- [ ] Confusing copy, evidence, controls, and source labeling are reviewed.
- [ ] Publish, revise, or stop decision is recorded with supporting observations.

Verification evidence: Pending.

## FT-404 — Deploy the public portfolio and preview environments

Status: Not started

Branch: `feat/ft-404-production-deployment`
Final commit: Pending
Pull request: Pending

Deploy the public Next.js interface on Vercel while placing the Rust API, optional continuous ingestion, and PostgreSQL/PostGIS on infrastructure suited to those persistent workloads.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [ ] Vercel project is connected to GitHub and creates isolated preview deployments for pull requests.
- [ ] Production Next.js environment calls the Rust API through a server-only configured URL.
- [ ] Rust API and continuous ingestion workers run on persistent container infrastructure with health checks and controlled releases.
- [ ] Managed PostgreSQL supports the required PostGIS extensions, backups, connection pooling, and region alignment.
- [ ] Secrets and environment variables are separated across development, preview, and production.
- [ ] Public domain, TLS, security headers, bounded logging, and basic availability monitoring are verified.
- [ ] Deployment, migration, rollback, and incident runbooks are tested.
- [ ] End-to-end smoke checks prove browser, API, database, replay fallback, source labeling, and degraded-state behavior.
- [ ] The public deployment contains no claim of certification, operational authority, commercial SLA, or real-operator endorsement.

Verification evidence: Pending.
