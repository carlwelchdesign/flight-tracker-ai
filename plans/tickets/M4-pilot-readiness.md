# M4 — Portfolio Launch and Demonstration Hardening

Default owner: Product and engineering, supported by security.

## FT-401 — Complete security, privacy, and trust review

Status: Complete

Branch: `docs/ft-401-security-trust-review`
Latest implementation commit: `e28ffa1`
Final commit: `e28ffa1`
Pull request: [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18)
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

Scope-reconciliation evidence (2026-07-21): FT-005 and ADR-010 remove commercial contract evidence, paid trials, pricing, SLA, and real-operator approval from this ticket. The implemented controls remain intact. `python3 scripts/validate_ft401_review.py --require-complete` passes after every residual risk is closed or controlled by a named downstream ticket and expiry; `validate_ft301_evidence.py --require-complete` is not an FT-401 completion condition. Closeout commit `e28ffa1` updates the portfolio limitation and evidence boundary; CI run [29851083689](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29851083689) passes Rust, web, and API/PostGIS checks.

## FT-402 — Run resilience and failure drills

Status: Complete

Branch: `feat/ft-402-resilience-drills`
Final implementation commit: `73e7157`
Pull request: [#22](https://github.com/carlwelchdesign/flight-tracker-ai/pull/22)
Owner: Backend, reliability, and full-stack engineering

Demonstrate a reliable recruiter-facing experience under feed, database, worker, network, and malformed-data failures.

Dependencies: FT-302, FT-304

Acceptance checklist:

- [x] Free-feed outage and high-latency checks produce visible degraded states and leave replay available.
- [x] Worker restart does not duplicate or lose lifecycle history beyond documented guarantees.
- [x] Database recovery procedure is tested.
- [x] Malformed and adversarial provider payloads are rejected or quarantined.
- [x] Alert backlog recovery behavior is measured.
- [x] Demo and developer runbooks are updated from the findings.

Verification evidence: focused Rust tests prove bounded timeout,
degraded/unavailable/recovered source transitions, malformed top-level failure,
invalid-record rejection, freshest-duplicate selection, and the one-megabyte
response cap. React behavior proves a timeout is visibly degraded and replay
remains directly selectable. [`RESILIENCE_DRILLS.md`](../RESILIENCE_DRILLS.md),
[`OPERATIONS_RUNBOOK.md`](../OPERATIONS_RUNBOOK.md),
[`BACKUP_RESTORE_RUNBOOK.md`](../BACKUP_RESTORE_RUNBOOK.md), and
[`PORTFOLIO_DEMO_RUNBOOK.md`](../PORTFOLIO_DEMO_RUNBOOK.md) record the measured
contract and honest recovery boundary. CI run
[29856364366](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29856364366)
passes Rust, web, and API/PostGIS checks. The PostGIS worker replacement keeps
one alert and its workflow-version-2 append-only comment; 208 queued batches
drain in 1,286 ms; a forced 273-batch overflow reports 257 skipped batches and
recovers from a complete replay window in 33 ms; and the isolated logical
restore verifies PostGIS, migration state, and exact controlled row counts in
3,520 ms with zero controlled transactions lost. These are CI rehearsal
measurements, not hosted SLA claims; FT-404 retains the representative managed
recovery gate.

## FT-403 — Validate the recruiter and hiring-manager demo

Status: In progress

Branch: `docs/ft-403-portfolio-demo-validation`
Latest implementation commit: `15227e4`
Final commit: Pending
Pull request: [#23](https://github.com/carlwelchdesign/flight-tracker-ai/pull/23)
Owner: Product design, user research, and engineering

Run a focused usability evaluation with representative recruiters, hiring managers, or neutral reviewers and explicit success/failure criteria.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [x] Demo scope, viewers, tasks, data modes, and prohibited operational uses are written.
- [x] Measures include time to understand the product, task completion, source-mode comprehension, and data availability.
- [ ] Users complete core workflows without facilitator intervention.
- [x] Confusing copy, evidence, controls, and source labeling are reviewed.
- [x] Publish, revise, or stop decision is recorded with supporting observations.

Verification evidence: [`PORTFOLIO_DEMO_VALIDATION.md`](../PORTFOLIO_DEMO_VALIDATION.md)
defines the unfacilitated protocol and records the current **Revise** decision.
The sequential expert simulation identified and corrected the missing H1 and
walkthrough, dispersed source-mode explanation, ambiguous outage control, and
missing alert fragment target. Focused component tests verify the orientation
contract. This evidence is explicitly not an independent human session; the
remaining participant checkbox and public **Publish** decision require a
neutral reviewer on the candidate preview produced by FT-404.

## FT-404 — Deploy the public portfolio and preview environments

Status: In progress

Branch: `feat/ft-404-production-deployment`
Latest implementation commit: `6a4929e`
Final commit: Pending
Pull request: [#24](https://github.com/carlwelchdesign/flight-tracker-ai/pull/24)
Owner: Platform, backend, security, and full-stack engineering

Deploy the public Next.js interface on Vercel while placing the Rust API, optional continuous ingestion, and PostgreSQL/PostGIS on infrastructure suited to those persistent workloads.

Dependencies: FT-401, FT-402

Acceptance checklist:

- [x] Vercel project is connected to GitHub and creates isolated preview deployments for pull requests.
- [x] Production Next.js environment calls the Rust API through a server-only configured URL.
- [x] Rust API and continuous ingestion workers run on persistent container infrastructure with health checks and controlled releases.
- [x] Managed PostgreSQL supports the required PostGIS extensions, backups, connection pooling, and region alignment.
- [x] Secrets and environment variables are separated across development, preview, and production.
- [x] Public domain, TLS, security headers, bounded logging, and basic availability monitoring are verified.
- [ ] Deployment, migration, rollback, and incident runbooks are tested.
- [ ] End-to-end smoke checks prove browser, API, database, replay fallback, source labeling, and degraded-state behavior.
- [x] The public deployment contains no claim of certification, operational authority, commercial SLA, or real-operator endorsement.

Verification evidence: The Vercel project `flight-tracker-ai` is Git-connected
to this repository with `apps/web` as its Next.js root and Node.js 20.x. The
first protected candidate deployment `dpl_BhRvwF9Bi5y67XW7w7qiQREaPrpj`
successfully built commit `03cbd31` and is not public: unauthenticated requests
are redirected to Vercel deployment protection. Preview deployment
`dpl_CHpF3CQacHMBJTnGhnpgsJtYLFfJ` independently built PR #24 commit `a2da158`
with the preview target and branch alias. CI run
[29858652637](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29858652637)
passes Rust, web, PostGIS, hosted-bootstrap, and sanitized FT-404 verifier tests.
[`HOSTED_DEPLOYMENT_RUNBOOK.md`](../HOSTED_DEPLOYMENT_RUNBOOK.md)
and [`render.yaml`](../../render.yaml) define the remaining Render, Neon, Clerk,
secret, restore, browser, and promotion gates. Hosted smoke evidence is pending.
On 2026-07-21, Vercel provisioned and attached available Neon resource
`neon-bronze-curtain` and Clerk resource `clerk-celeste-door` to
`flight-tracker-ai`. The expected encrypted environment-variable names are
present across Development, Preview, and Production. The Neon pooled and direct
TLS paths target AWS `us-east-1`, and the direct path successfully enabled and
reported PostGIS `3.5.0`. The auth mode, operations mode, assertion key ID,
issuer, and audience are configured across all three Vercel environments; the
API URL and internal assertion secret wait for Render. Render remains
unprovisioned, so its Blueprint region is aligned to Virginia before service
creation. Clerk organization/user
bootstrap, Neon snapshot/isolated restore, Render deployment, cross-service
secrets, and hosted smoke remain pending.
Provisioning and region-alignment evidence is recorded in commit `f897532` and
draft PR [#24](https://github.com/carlwelchdesign/flight-tracker-ai/pull/24).

The first public-alias observation exposed two controlled setup defects rather
than a usable candidate: the original deployment predated the Clerk variables,
and the five non-secret production/preview runtime settings had been stored as
the literal placeholder `[SENSITIVE]` by noninteractive CLI input. The Vercel
values are corrected and verified by an environment pull. The portfolio root
now remains public long enough to present a safe sign-in state while operational
and backend routes stay behind Clerk, and hosted 500-level configuration errors
are replaced with bounded evaluation copy instead of disclosing variable names.
The Clerk production domain is now `flight-tracker-ai-one.vercel.app`.
Production uses Clerk live keys while Preview retains test keys. Organizations
with required membership are enabled and `Flight Tracker Portfolio` exists;
reviewer enrollment remains a manual identity step. Exact commit `e33a21c` was
built as protected preview deployment `dpl_5jVqrumWgwHJiUBc4i4fHZkdo6LL` and
production deployment `dpl_2hfw56Se2W9oSDx7fCQ4F3hHd2cb`. The public alias
returns HTTP 200 with `Sign in to continue`, `/sign-in` renders the production
Clerk flow without browser console errors, the `DEV_AUTH_SUBJECT` failure is
absent, and the expected security headers are present. Render and the full
cross-service hosted smoke remain pending.

The 2026-07-21 Render specification audit found that managed preview
environments require a Pro workspace and omit `sync: false` secrets. To retain
the zero-base-cost and environment-isolation requirements, `render.yaml` now
defines explicit free staging and production services. Staging follows passing
feature-branch checks during FT-404 verification; production is promoted
manually after staging and browser smoke. The final promotion commit switches
both services to `main`. Render Free rejects a configurable maximum shutdown
delay, so the Blueprint uses the platform default.
Each service requires a distinct Neon database URL and internal assertion secret.
The Blueprint passes Render's official JSON Schema, the Rust release build
passes locally, and CI run
[29862882559](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29862882559)
passes all repository jobs at commit `e33a21c`.

Render Blueprint `exs-d9ft018okrbs738q5r60` created production service
`srv-d9ft2gn7f7vs739ass40` and staging service
`srv-d9ft2gn7f7vs739ass3g`. Production uses `neon-bronze-curtain`; staging uses
the separately provisioned `neon-bistre-lantern` Free database attached only to
Preview under namespaced variables. Preview and Production have distinct API
origins and internal assertion secrets. The first staging launch failed closed
when a sensitive Vercel pull yielded the literal placeholder `[SENSITIVE]`;
the authenticated provider value was applied directly without exposing it.
Production health and migrations then exposed an idle alert-worker heartbeat
defect through the protected diagnostics route. Commit `a665494` adds the
periodic heartbeat, API HSTS, and bounded signed-out verifier contract; commit
`6a4929e` passes CI run
[29865574640](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29865574640).

Render production deploy `dep-d9ftbi61a83c7396hbsg` and staging deploy
`dep-d9ftf2naqgkc738okfig` run commit `6a4929e`. Both return exact health and
readiness responses with HSTS, PostGIS and migrations ready, and
`alert_projection`, `fleet_projection`, `replay_runtime`, and
`retention_scheduler` running. Distinct short-lived internal assertions passed
in both environments; their temporary verifier identities were removed.
Vercel preview `dpl_FNbngNWmbKNSafY5rvNypHjPaPzS` passes the sanitized protected
preview contract, and refreshed production deployment
`dpl_FXv3uAUVCKCRTTfTm5xRj7rn1pWE` passes the publication-ready public-boundary
verifier. Commit `790e022` then corrects a strict-CSP regression by opting
Clerk's provider into dynamic rendering so the browser script receives the
request nonce. CI run
[`29867545134`](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29867545134)
passes all jobs. Production deployment `dpl_9Mtv1MzkUR9swi8ycM7yDga1e3RM` is
the current public alias; its Clerk script includes the nonce and `/sign-in`
visibly renders the production email/password form. Reviewer enrollment and
authenticated browser/FT-401 smoke remain open.

On 2026-07-21, Neon retained the manual production snapshot
`main at 2026-07-21 20:46:51 UTC (manual)` with no expiry. A temporary isolated
branch restored `main` from 13:45 PDT and matched production with PostGIS
`3.5.0`, 14 successful SQLx migrations, one operator, and zero identities,
memberships, alerts, or alert actions. An earlier 13:05 PDT restore candidate
predated the application schema and therefore failed the migration release
gate as designed. Both temporary branches were deleted after verification;
production `main` was not modified and no connection string was recorded.
