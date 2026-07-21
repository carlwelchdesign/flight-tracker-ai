# M4 — Portfolio Launch and Demonstration Hardening

Default owner: Product and engineering, supported by security.

## FT-401 — Complete security, privacy, and trust review

Status: In progress

Branch: `docs/ft-401-security-trust-review`
Latest implementation commit: `4649408`
Final commit: Pending
Pull request: [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18) (draft; rebase and re-evaluate against portfolio scope after FT-005)

Review data handling, permissions, auditability, portfolio language, and source obligations before making the demonstration public.

Dependencies: FT-303, FT-304

Acceptance checklist:

- [ ] Threat model covers credentials, tenant isolation, ingestion abuse, and privileged actions.
- [ ] Retention, deletion, backup, and incident-response policies are documented.
- [ ] Portfolio-only and not-for-operational-use language is consistent across normal, loading, empty, degraded, and error states.
- [ ] Any enabled free source's attribution, rate-limit, caching, and retention obligations are implemented; replay-only deployment remains valid.
- [ ] Security findings have owners and deadlines.

Verification evidence: draft PR [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18) contains the implemented threat model, trust controls, retention workflows, audit review, credential rotation, public-probe hardening, and hosted-drill tooling. After FT-005 merges, rebase the branch, remove commercial-provider and real-operator completion gates, and verify the remaining controls against a public portfolio deployment.

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
