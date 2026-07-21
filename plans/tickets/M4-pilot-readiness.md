# M4 — Pilot Readiness and Operational Hardening

Default owner: Product/operations lead, supported by engineering and security.

## FT-401 — Complete security, privacy, and trust review

Status: In progress

Branch: `docs/ft-401-security-trust-review`
Latest implementation commit: `d975ac3`
Pull request: [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18) (draft; do not merge while completion gate fails)
Owner: Security, legal/privacy, product, and engineering

Review data handling, permissions, auditability, advisory language, and external-provider obligations before a real-operations evaluation.

Dependencies: FT-301, FT-303, FT-304

Preparation scope: review the implemented FT-303/FT-304 boundaries now; keep provider-specific attribution, retention, deletion, and licensing controls open until FT-301 supplies the controlling contract.

Preparation checklist:

- [x] Audit implemented authentication, tenancy, ingestion, audit, privileged-action, and advisory-language boundaries.
- [x] Create the threat model and proposed data lifecycle, backup, and incident-response baseline.
- [x] Record severity, owner, deadline gate, treatment, and verification for every finding.
- [x] Bound and scan sensitive user-written operational fields without returning their content.
- [ ] Resolve FT-301 and implement the selected provider's controlling obligations.

Acceptance checklist:

- [x] Threat model covers credentials, tenant isolation, ingestion abuse, and privileged actions.
- [x] Retention, deletion, backup, and incident-response policies are documented.
- [ ] Advisory-only product language is reviewed and consistent.
- [ ] Provider attribution and licensing obligations are implemented.
- [x] Security findings have owners and deadlines.

Verification evidence: [`SECURITY_PRIVACY_TRUST_REVIEW.md`](../SECURITY_PRIVACY_TRUST_REVIEW.md), [`DATA_LIFECYCLE_INCIDENT_POLICY.md`](../DATA_LIFECYCLE_INCIDENT_POLICY.md), [`CREDENTIAL_ROTATION_RUNBOOK.md`](../CREDENTIAL_ROTATION_RUNBOOK.md), [`AUDIT_REVIEW_RUNBOOK.md`](../AUDIT_REVIEW_RUNBOOK.md), [`RETENTION_DELETION_RUNBOOK.md`](../RETENTION_DELETION_RUNBOOK.md), [`BACKUP_RESTORE_RUNBOOK.md`](../BACKUP_RESTORE_RUNBOOK.md), [`SECURITY_FINDINGS.csv`](../SECURITY_FINDINGS.csv), and `python3 scripts/validate_ft401_review.py`. Structural validation passes; `--require-complete` intentionally fails while critical/high findings and the FT-301 provider gate remain open. CI run [29833385671](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29833385671) closes F401-004 through operator-scoped assignment constraints. CI run [29834083229](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834083229) closes F401-010 through exact minimal public probes and authenticated diagnostics against PostGIS. CI run [29834813131](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834813131) verifies the named assertion rotation protocol and browser-asset secret scan; F401-001 remains open for managed hosted secrets and completed drills. Commit `54db2b1` adds a root-level, mode-aware advisory/source-authority banner that remains present across operational states; 36 web tests, lint, typecheck, and a production build pass locally. Commit `b022142` adds administrator-only tenant audit review, bounded redacted CSV export, and deterministic high-risk/burst signals; CI run [29836717456](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29836717456) passes Rust, web, and PostGIS contracts. Commit `f5ba6c1` adds approved raw-payload retention and tombstone suppression; CI run [29837700053](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29837700053) proves the migration, two-person workflow, scope preservation, deletion audit, and simulated restore suppression. Commit `fdf75af` extends the same workflow to authorization audit, expired session revocations, and exclusive inactive identity minimization; CI run [29838690551](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29838690551) proves tenant scope, second-person approval, safe shared/current identity preservation, deletion/minimization, and restore suppression. Commit `b937f8a` adds whole-terminal-alert-series retention, dependency-ordered action/evidence deletion, logical replay tombstones, and safe next-revision continuation; CI run [29839592816](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29839592816) passes Rust, web, and PostGIS contracts. Commit `420f145` adds provider-scoped normalized-fact retention for old observations, whole terminal flights, and whole expired hazard series; CI run [29840281133](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29840281133) proves dependency ordering, provider/tenant isolation, active/reference preservation, and restore suppression. Commit `5c320b8` adds separately approved exact-policy schedules, durable attempts/failures, drift-free idempotent cadence, and a supervised Rust worker; CI run [29841291067](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29841291067) proves automatic deletion, duplicate-slot suppression, tenant scope, inactive-actor fail-closed behavior, and API/BFF contracts. Commit `ec9ea2a` adds administrator-only tenant resurrection diagnostics and the controlled restore procedure; CI run [29841960517](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29841960517) proves healthy post-lifecycle state, deliberate tombstone-conflict detection, tenant isolation, and all Rust/web/PostGIS contracts. F401-002 remains open for FT-301 provider values, shared-identity disposition, and managed execution; F401-007 remains open for the hosted retention/integrity incident drill; F401-008 remains open for managed backup configuration and the recorded restore drill. F401-009 remains open for exact Product/Legal/operator wording approval and a real preview state review. F401-005 remains open for a real Clerk preview browser smoke.

Exact-inventory evidence: commit `a521e91` binds every new retention run to a SHA-256 fingerprint of its eligible record keys under repeatable-read execution. CI run [29843308802](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29843308802) proves a same-count record substitution is rejected without deletion and that restoring the original inventory reproduces its fingerprint.

Bounded-write evidence: commit `cd08649` applies matching Rust and PostgreSQL limits to dispatcher notes, action identifiers, and session-revocation reasons, normalizes idempotency before locking and lookup, returns typed 422 errors, and exposes the 2,000-character note limit in the console. CI run [29844162906](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29844162906) proves API rejection, direct-database enforcement, 71 Rust library tests, 43 web tests, lint, typecheck, production build, and the PostGIS contract.

Sensitive-write monitoring evidence: commit `d975ac3` adds a deterministic Rust policy that scans bounded dispatcher comments and session-revocation reasons for credential material or personal email addresses inside the administrator-only tenant boundary. Signals disclose only severity, actor, time, record ID, and field class. CI run [29845085036](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29845085036) proves controlled credential/email detection, ordinary aviation-text rejection, response non-leakage, cross-tenant isolation, 75 Rust library tests, 43 web tests, strict lint/type checks, production build, and the fresh PostGIS contract. F401-007 now remains open only for representative hosted execution of the audit/retention incident drill.

## FT-402 — Run resilience and failure drills

Status: Not started

Branch: `feat/ft-402-resilience-drills`
Final commit: Pending
Pull request: Pending

Demonstrate safe behavior under provider, database, worker, network, and malformed-data failures.

Dependencies: FT-302, FT-304

Acceptance checklist:

- [ ] Provider outage and high-latency drills produce visible degraded states.
- [ ] Worker restart does not duplicate or lose lifecycle history beyond documented guarantees.
- [ ] Database recovery procedure is tested.
- [ ] Malformed and adversarial provider payloads are rejected or quarantined.
- [ ] Alert backlog recovery behavior is measured.
- [ ] Operator and developer runbooks are updated from drill findings.

Verification evidence: Pending.

## FT-403 — Validate a limited advisory pilot

Status: Not started

Branch: `docs/ft-403-advisory-pilot-validation`
Final commit: Pending
Pull request: Pending

Run a controlled evaluation with representative users and explicit success/failure criteria.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [ ] Pilot scope, users, duration, data, and prohibited uses are written and approved.
- [ ] Baseline measures include detection time, response time, duplicate rate, dismissal rate, and data availability.
- [ ] Users complete core workflows without facilitator intervention.
- [ ] False positives, false negatives, and confusing evidence are reviewed.
- [ ] Go, revise, or stop decision is recorded with supporting data.

Verification evidence: Pending.

## FT-404 — Deploy the production system and preview environments

Status: Not started

Branch: `feat/ft-404-production-deployment`
Final commit: Pending
Pull request: Pending

Deploy the public Next.js interface on Vercel while placing persistent Rust ingestion and API workloads on appropriate container infrastructure with managed PostgreSQL/PostGIS.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [ ] Vercel project is connected to GitHub and creates isolated preview deployments for pull requests.
- [ ] Production Next.js environment calls the Rust API through a server-only configured URL.
- [ ] Rust API and continuous ingestion workers run on persistent container infrastructure with health checks and controlled releases.
- [ ] Managed PostgreSQL supports the required PostGIS extensions, backups, connection pooling, and region alignment.
- [ ] Secrets and environment variables are separated across development, preview, and production.
- [ ] Production domain, TLS, security headers, logging, tracing, and alerting are verified.
- [ ] Deployment, migration, rollback, and incident runbooks are tested.
- [ ] End-to-end smoke checks prove browser, API, database, and degraded-state behavior.

Verification evidence: Pending.
