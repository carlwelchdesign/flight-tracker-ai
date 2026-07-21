# Data Lifecycle, Backup, and Incident Policy

FT-401 portfolio control baseline, last reviewed 2026-07-21. Repository controls are implemented and verified as cited; environment-specific scheduling, backups, and drills remain FT-402/FT-404 deployment gates. Any future external source's stricter official term wins.

## Data lifecycle policy

| Data | Proposed maximum active retention | Deletion/exit behavior | Status |
| --- | --- | --- | --- |
| ADSB.lol raw aircraft-position messages and normalized positions | Zero persistent retention | Keep only transient in-process current state; never write to PostgreSQL, files, browser storage, logs, analytics, exports, AI inputs, fixtures, or backups. | FT-302 enforces an ephemeral, disabled-by-default regional adapter and `no-store` API/web responses. |
| Future external aircraft-position messages | Source terms or 24 hours, whichever is shorter | Do not enable until a source-specific policy and deletion proof are approved. | Generic enforcement exists, but no other aircraft-position source is enabled. |
| NOAA raw public observations | 30 days for debugging/replay | Delete expired raw payloads; normalized current/revision records may follow operational history policy. | Raw-payload and normalized-fact policy/preview/approval/deletion/tombstone enforcement plus separately approved scheduling are implemented. |
| Flight positions/routes/status facts | 30 days for the portfolio unless source terms require shorter | Tenant deletion plus source-specific deletion; exclude expired facts from restore. | Two-person source-scoped deletion of whole unreferenced terminal flight aggregates and separately approved scheduling are implemented; FT-404 configures the hosted schedule. |
| Alerts, user actions, and rule evidence | 12 months for portfolio evaluation, subject to source terms | Preserve append-only decision evidence; redact/delete prohibited source fields; record any security hold. | Notes and action identifiers are write-bounded; two-person whole-terminal-series deletion, logical replay suppression, and separately approved scheduling are implemented; FT-404 configures hosted execution. |
| Authorization audit events | 12 months | Preserve actor/action/target evidence; restrict exports; delete on approved schedule, not user UI action. | Two-person preview/approval/deletion, restore-suppression tombstones, and separately approved scheduling implemented; hosted execution remains pending. |
| Membership and identity mapping | Active relationship plus 30 days | Revoke access immediately; later delete/minimize external identifiers while preserving lawful audit references. | Revocation and two-person minimization of exclusively tenant-owned inactive identities implemented; shared-identity disposition and scheduling remain pending. |
| Session revocation records | Expiry plus 30 days | Scheduled deletion after investigation window unless security hold applies. | Expiry enforcement, two-person cleanup, restore suppression, and separately approved scheduling implemented. |
| Application/security logs | 30 days hot; no request bodies, assertions, credentials, raw source payloads, or sensitive identifiers | Restrict access; redact at ingestion; shorten if source terms require. | Logging minimization is implemented in application boundaries; FT-404 configures hosted retention. |
| Metrics | 13 months only when aggregate/non-identifying and contract-approved | Delete tenant/source labels that exceed approved aggregation. | Current metrics are in-memory; hosted policy pending. |
| Optional future commercial records | Per applicable record schedule outside Git | External record-system disposition; Git retains only opaque references/redacted summaries. | Archived production-track process; not created for the portfolio release. |

No code may silently extend a period. Retention configuration must be field/source aware, versioned, auditable, and testable with a dry run. FT-404 records the actual hosted configuration before publication. The implemented raw-payload and application-lifecycle workflow and remaining limits are in `RETENTION_DELETION_RUNBOOK.md`.

## Deletion requirements

1. Maintain an inventory from source envelope through normalized facts, alerts/evidence, exports, logs, caches, and backups.
2. Trigger deletion for expiry, contract termination, provider request, operator revocation, aircraft entitlement loss, privacy restriction, and security response.
3. Use tenant/source/time predicates with preview counts and a second-person approval for bulk deletion.
4. Write a non-payload deletion audit containing policy version, scope, counts, requester/approver, start/end time, failures, and controlled evidence reference.
5. Propagate tombstones or an equivalent suppression list into restores so deleted data is not resurrected.
6. Never claim complete deletion until primary, replica, cache, export, and backup expiry obligations are accounted for.

## Backup and recovery baseline

- Encrypt database backups and snapshots in transit and at rest with access separated from application runtime credentials.
- Keep production and preview backups isolated by environment and tenant exposure.
- Proposed backup retention is 35 days, shortened when a provider contract requires it.
- Test restore before persistent public deployment and after material backup changes; record RPO, RTO, migration version, PostGIS availability, row/tenant counts, and tombstone replay.
- A restore is incomplete until current membership/session revocations and deletion tombstones are reapplied before serving traffic.
- Follow `BACKUP_RESTORE_RUNBOOK.md` and require a healthy administrator-only retention-integrity result for every tenant before serving traffic. The controlled procedure and integrity gate are implemented; managed backup configuration and a recorded drill remain pending.
- Provider credentials and hosted identity secrets are restored from the managed secret system, never from database backups.

## Security incident classification

| Severity | Examples | Initial control objective |
| --- | --- | --- |
| SEV-1 | Cross-tenant disclosure, credential compromise with active access, prohibited aircraft exposure, unsafe authoritative presentation, destructive data loss | Stop exposure/ingestion immediately; revoke/rotate; preserve evidence; notify executive, Security, Legal, Product, affected operator, and provider according to controlling obligations. |
| SEV-2 | Confirmed unauthorized attempt, material feed poisoning, prolonged loss of audit/freshness controls, deletion failure without known disclosure | Contain affected tenant/source; disable risky action; preserve evidence; begin scoped investigation and contractual notification assessment. |
| SEV-3 | Low-impact policy violation, blocked suspicious activity, non-sensitive availability issue | Correct, document, trend, and promote severity if scope changes. |

Any source-specific or legal notification requirement is established when that source or hosted service is selected. Absence of a known deadline must never delay immediate containment.

## Incident response sequence

1. **Detect and declare:** assign incident commander, severity, timestamp, affected tenants/sources/environments, and a controlled incident reference.
2. **Contain:** revoke sessions, rotate compromised credentials, disable provider ingestion or privileged actions, isolate exports, and preserve the last safe advisory state with a visible degraded/unavailable label.
3. **Preserve evidence:** capture correlation IDs, audit rows, configuration/migration versions, hashes, timestamps, and minimal logs. Do not paste raw licensed data, secrets, real tails, or personal data into chat or public tickets.
4. **Assess:** determine data classes, tenant/provider scope, operational impact, deletion/retention conflict, and whether source facts or generated wording were exposed or altered.
5. **Notify:** Security and Legal decide provider/operator/user/regulator notice using controlling terms. Product approves operational status messaging; engineering does not improvise legal notice.
6. **Eradicate and recover:** patch or disable the cause, rotate/revoke, validate tenant boundaries and data freshness, restore with tombstones, and require a second-person recovery approval.
7. **Learn:** publish a redacted timeline/root cause, create owned findings, test the regression/failure drill, and update the threat model/runbooks before closing.

## RACI and access

| Activity | Security | Legal/privacy | Engineering | Product/operations | Operator/provider |
| --- | --- | --- | --- | --- | --- |
| Declare severity and contain technical access | A | C | R | C | I |
| Determine contractual/privacy notification | C | A/R | C | C | C/I |
| Preserve evidence and recover service | A | C | R | C | I |
| Approve external operational messaging | C | C | I | A/R | C |
| Close incident and accept residual risk | A | required approval | R | required approval | C |

Only named incident responders may access restricted incident evidence. Access, exports, and deletion must be audited. Repository controls are accepted by FT-401; FT-402 exercises recovery and incident behavior, and FT-404 verifies the selected hosted environment before publication.
