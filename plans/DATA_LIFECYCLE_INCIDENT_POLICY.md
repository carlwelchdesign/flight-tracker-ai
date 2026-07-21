# Data Lifecycle, Backup, and Incident Policy

FT-401 proposed pilot control baseline, last reviewed 2026-07-21. These controls are mandatory gates, not claims about currently implemented jobs or infrastructure. The stricter controlling provider term always wins.

## Data lifecycle policy

| Data | Proposed maximum active retention | Deletion/exit behavior | Status |
| --- | --- | --- | --- |
| Commercial raw provider messages | 24 hours unless the controlling Order expressly permits longer | Delete primary objects and derived raw caches; preserve only contract-approved normalized/audit fields; record deletion evidence. | Generic raw-payload enforcement exists; provider policy approval remains blocked on FT-301. |
| NOAA raw public observations | 30 days for debugging/replay | Delete expired raw payloads; normalized current/revision records may follow operational history policy. | Raw-payload policy/preview/approval/deletion/tombstone enforcement implemented; policy approval and normalized-record retention remain pending. |
| Flight positions/routes/status facts | 30 days for a limited pilot unless contract/operator requires shorter | Tenant deletion plus provider-entitlement deletion; exclude expired facts from restore. | Not implemented. |
| Alerts, dispatcher actions, and rule evidence | 12 months for pilot evaluation, subject to provider derivative rights | Preserve append-only decision evidence; redact/delete prohibited source fields; Legal approves any litigation/security hold. | Not implemented. |
| Authorization audit events | 12 months | Preserve actor/action/target evidence; restrict exports; delete on approved schedule, not user UI action. | Two-person preview/approval/deletion and restore-suppression tombstones implemented; approved scheduling and hosted execution remain pending. |
| Membership and identity mapping | Active relationship plus 30 days | Revoke access immediately; later delete/minimize external identifiers while preserving lawful audit references. | Revocation and two-person minimization of exclusively tenant-owned inactive identities implemented; shared-identity disposition and scheduling remain pending. |
| Session revocation records | Expiry plus 30 days | Scheduled deletion after investigation window unless security hold applies. | Expiry enforcement plus two-person cleanup and restore suppression implemented; scheduling remains pending. |
| Application/security logs | 30 days hot; no request bodies, assertions, credentials, raw provider payloads, or real tail lists | Restrict access; redact at ingestion; shorten if provider/operator terms require. | Logging minimization partially implemented; hosted retention pending. |
| Metrics | 13 months only when aggregate/non-identifying and contract-approved | Delete tenant/source labels that exceed approved aggregation. | Current metrics are in-memory; hosted policy pending. |
| Controlled contracts, quotes, approvals | Per legal/procurement record schedule outside Git | Contract repository disposition; Git retains only opaque references/redacted summaries. | Process defined in FT-301. |

The Product, Legal/privacy, Security, and operator owners must approve or shorten this baseline before a pilot. No code may silently extend a period. Retention configuration must be field/source aware, versioned, auditable, and testable with a dry run. The implemented raw-payload and application-lifecycle workflow and remaining limits are in `RETENTION_DELETION_RUNBOOK.md`.

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
- Test restore at least quarterly and before a pilot; record RPO, RTO, migration version, PostGIS availability, row/tenant counts, and tombstone replay.
- A restore is incomplete until current membership/session revocations and deletion tombstones are reapplied before serving traffic.
- Provider credentials and hosted identity secrets are restored from the managed secret system, never from database backups.

## Security incident classification

| Severity | Examples | Initial control objective |
| --- | --- | --- |
| SEV-1 | Cross-tenant disclosure, credential compromise with active access, prohibited aircraft exposure, unsafe authoritative presentation, destructive data loss | Stop exposure/ingestion immediately; revoke/rotate; preserve evidence; notify executive, Security, Legal, Product, affected operator, and provider according to controlling obligations. |
| SEV-2 | Confirmed unauthorized attempt, material feed poisoning, prolonged loss of audit/freshness controls, deletion failure without known disclosure | Contain affected tenant/source; disable risky action; preserve evidence; begin scoped investigation and contractual notification assessment. |
| SEV-3 | Low-impact policy violation, blocked suspicious activity, non-sensitive availability issue | Correct, document, trend, and promote severity if scope changes. |

Contractual/regulatory notification clocks are pending FT-301 and legal review. Absence of a known deadline must never delay immediate internal escalation.

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

Only named incident responders may access restricted incident evidence. Access, exports, and deletion must be audited. F401-002, F401-007, and F401-008 remain open until these policies are implemented and exercised by FT-402.
