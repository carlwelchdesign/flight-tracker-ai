# Backup Restore and Tombstone Integrity Runbook

This runbook defines the FT-401/F401-008 recovery procedure for PostgreSQL/PostGIS. It does not claim that managed backups exist or that a hosted drill has passed. A restore is not service-ready until current authorization state and every deletion tombstone are newer than or equal to the restored application data and the integrity check is clean.

## Required controls before a pilot

- Managed PostgreSQL backups and snapshots are encrypted in transit and at rest, isolated by environment, access-logged, and unavailable to normal application credentials.
- Point-in-time recovery and snapshot retention are no longer than 35 days or the shorter controlling provider period.
- A separately protected control copy captures current operator memberships, disabled identities, session revocations, retention policies/runs/schedules/attempts, and all four tombstone tables after every approved lifecycle operation.
- At least two named responders can restore; one performs the procedure and another approves traffic restoration.
- The restore target cannot receive browser/API/worker traffic until the release gate below passes.

## Restore sequence

1. Declare a controlled recovery or incident reference. Record requester, approver, source backup timestamp, target environment, expected migration version, and proposed RPO/RTO.
2. Create an isolated target network/database with ingestion, scheduler, API, and public routing disabled.
3. Restore the selected encrypted backup. Record provider job IDs and timestamps outside Git; never paste credentials, payloads, session IDs, or real tail lists into evidence.
4. Apply repository migrations through the exact candidate application release. Verify PostgreSQL and PostGIS versions before continuing.
5. Import the latest protected authorization/control delta in dependency order: identities and memberships, session revocations, retention policies and runs, schedules and attempts, then raw/lifecycle/alert-history/operational-fact tombstones. Resolve conflicts in favor of the newer denial, revocation, deletion, or minimized state.
6. Keep application traffic disabled. Start one internal API instance and the retention scheduler only after the current control delta is present.
7. As a tenant administrator, call `GET /api/admin/retention/integrity` for every restored tenant. Every value under `violations` must be zero. Tombstone counts must match the protected control-copy manifest.
8. Investigate every paused schedule and scheduled failure. A failure is not a tombstone violation, but it must have an owned disposition before traffic resumes.
9. Re-run representative exact-replay probes for raw payload, authorization audit, session revocation, identity minimization, alert history, and normalized facts. The deleted/minimized state must remain effective, while a genuinely new material alert/fact revision remains possible.
10. Verify current membership denial, disabled identity denial, session revocation, tenant isolation, PostGIS queries, minimal public probes, authenticated diagnostics, and one read-only operational workflow.
11. Measure achieved RPO/RTO and compare restored tenant/source/row counts with the approved manifest. Security and the recovery approver sign the release decision before routing, workers, and ingestion are enabled.

If any integrity violation is non-zero, stop. Do not delete the conflicting evidence ad hoc and do not open traffic. Preserve the isolated target, identify whether the backup or control delta is stale, correct the restore input, and repeat from a new isolated target.

## Rollback and abort

- If migrations, PostGIS, control-delta import, integrity checks, or authorization probes fail, destroy or quarantine only the isolated restore target according to the managed-provider procedure; do not modify the known production source.
- If the selected backup cannot satisfy the approved RPO or provider deletion deadline, escalate as SEV-2 or SEV-1 based on exposure/availability impact.
- If tombstones are newer than their referenced retention runs in the restored backup, import the matching run/policy evidence before tombstones; never weaken foreign keys to force recovery.
- If a revoked administrator owned an active schedule, leave it paused and create a newly reviewed schedule with two active administrators after recovery.

## Required drill evidence

- [ ] Managed backup encryption, isolation, retention, and access controls are captured by controlled references.
- [ ] Restore source/target, migration and PostGIS versions, participants, timestamps, and approvals are recorded.
- [ ] Current authorization/control delta and tombstone manifest are reapplied before traffic.
- [ ] Every tenant integrity response is healthy with zero violations.
- [ ] Exact replay remains suppressed for each implemented lifecycle class.
- [ ] Current/referenced/new-material records remain available as designed.
- [ ] Membership/session/tenant probes fail closed.
- [ ] RPO and RTO are measured against the approved objectives.
- [ ] Paused schedules/failures and every deviation have an owner and disposition.
- [ ] Security and the recovery approver sign the go/no-go decision.

F401-008 remains open until this checklist is completed in the representative managed environment. FT-402 owns the destructive recovery drill; this runbook supplies its release gate.

## Repository recovery rehearsal

FT-402 automates the non-hosted portion of this sequence in CI with
`scripts/run_ft402_database_recovery.sh`. Against the ephemeral PostGIS service,
it writes one controlled marker, captures a custom-format logical backup,
restores to a distinct database ending in `_ft402_restore`, and verifies the
marker, alert/action counts, successful migration ledger, and PostGIS extension.
It prints measured controlled-snapshot RPO/RTO and then removes the scratch
target and archive.

This proves that the schema and repository data can traverse the documented
dump/restore boundary. It does not check managed encryption, provider snapshot
retention, protected control-copy import, access logs, regional isolation,
human approvals, or a traffic cutover. Those required drill-evidence boxes
remain unchecked until FT-404 exercises a representative hosted environment.
