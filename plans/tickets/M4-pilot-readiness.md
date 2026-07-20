# M4 — Pilot Readiness and Operational Hardening

Default owner: Product/operations lead, supported by engineering and security.

## FT-401 — Complete security, privacy, and trust review

Status: Not started

Branch: `docs/ft-401-security-trust-review`
Final commit: Pending
Pull request: Pending

Review data handling, permissions, auditability, advisory language, and external-provider obligations before a real-operations evaluation.

Dependencies: FT-301, FT-303, FT-304

Acceptance checklist:

- [ ] Threat model covers credentials, tenant isolation, ingestion abuse, and privileged actions.
- [ ] Retention, deletion, backup, and incident-response policies are documented.
- [ ] Advisory-only product language is reviewed and consistent.
- [ ] Provider attribution and licensing obligations are implemented.
- [ ] Security findings have owners and deadlines.

Verification evidence: Pending.

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
