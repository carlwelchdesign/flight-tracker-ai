# Audit Review and Privileged-Action Monitoring

This runbook governs the tenant-scoped audit review surface added for FT-401. It supports investigation and evidence export; it is not a substitute for a SIEM, database backup, or approved retention schedule.

## Access boundary

- Only an authenticated application `administrator` has the named `ReviewAudit` permission.
- Every query derives `operator_id` from the verified Rust `AuthContext`; neither the browser nor query parameters can select another tenant.
- `GET /api/admin/audit-events` returns at most 250 redacted events. Its default window is the prior 24 hours.
- `GET /api/admin/audit-alerts` evaluates at most the prior 24 hours and fails rather than silently truncating more than 10,000 events.
- `GET /api/admin/audit-events/export` requires an explicit ordered range of at most 31 days and fails rather than exporting more than 10,000 events.

The Next.js BFF allowlists only these exact audit paths and preserves the fixed CSV download header.

## Redaction and export rules

The review API joins authorization audit events and alert actions but returns only an allowlist of operational fields. It excludes:

- free-form alert comments;
- free-form session-revocation reasons;
- idempotency keys;
- raw hosted session identifiers; and
- unrecognized authorization metadata.

Membership role/status, provider name, identity reference, structured dismissal reason, and assignment reference may be included. CSV cells are quoted and formula-leading values are neutralized. Exports use `Cache-Control: no-store` and must remain in the operator's approved evidence system.

## Monitoring policy

The monitor emits one warning for every high-risk event:

- session revocation;
- membership revocation or administrator promotion;
- alert dismissal; or
- alert resolution.

It emits a critical `privileged_action_burst` signal when one actor performs three sensitive/high-risk actions within 15 minutes. Signals are deterministic views of persisted evidence; operators must still assess intent and impact.

## Review procedure

1. Open the administrator audit review below the operations console.
2. Confirm the tenant/operator shown in the main console is the intended scope.
3. Review critical burst signals first, then individual high-risk events.
4. Correlate actor, action, target type/reference, and UTC time with the membership or alert record. Use restricted database access for excluded content only when incident policy authorizes it.
5. Export the smallest necessary time range. Record the recipient, purpose, storage location, and deletion deadline outside the CSV.
6. Escalate unexplained privileged changes under the severity table in `DATA_LIFECYCLE_INCIDENT_POLICY.md`.

## Pre-pilot incident drill

F401-007 cannot close until Security and the operator owner complete this drill in a representative hosted environment:

- [ ] Create one approved administrator promotion and one session revocation.
- [ ] Create three sensitive/high-risk actions by one test actor within 15 minutes.
- [ ] Confirm the expected warning and critical signals appear only in the correct tenant.
- [ ] Confirm a viewer and operator receive `403` from review, export, and monitoring routes.
- [ ] Export a bounded CSV and verify no comment, revocation reason, idempotency key, or session ID appears.
- [ ] Follow the incident escalation path and record the disposition.
- [ ] Verify the approved retention job and integrity procedure from F401-002 have run against the same evidence class.

Record environment, participants, timestamps, test actor/tenant references, screenshots or controlled evidence links, results, deviations, and remediation tickets. Do not place real session IDs, secrets, or unrestricted exports in Git.
