# Retention and Deletion Runbook

This runbook describes the implemented FT-401 lifecycle-retention workflow. It is deny-by-default: no data is deleted or minimized until a tenant administrator creates a versioned policy, a different administrator approves it, a fixed inventory is previewed, and a different administrator approves that run.

## Implemented scope

The current engine supports six data classes:

- `provider_raw_payload` replaces expired `provider_envelopes.raw_payload` values with an empty object while preserving non-payload envelope identity, provider/feed, timing, SHA-256 evidence, and normalized records.
- `authorization_audit` deletes expired tenant authorization-audit rows after recording lifecycle tombstones.
- `session_revocation` deletes expired revocation rows only after the approved retention interval has elapsed beyond session expiry.
- `identity_mapping` minimizes an inactive identity only when its old revoked membership belongs exclusively to the current tenant, no active or newer membership exists, and the identity is not shared with another tenant. Minimization replaces the external subject with an opaque deleted identifier, clears the display name, and preserves the original disabled time.
- `terminal_alert_history` deletes an entire alert series only when every revision is dismissed or resolved and the latest alert/action activity is older than the cutoff. Actions and evidence are deleted before their parent alerts; open, acknowledged, recent, mixed-state, and cross-tenant series remain untouched.
- `normalized_operational_fact` is provider-scoped. It deletes old airport observations independently; deletes a terminal landed/cancelled flight only with all same-provider old routes/positions and no alert reference; and deletes a whole expired hazard series only when every revision is from the scoped provider and no alert references the series.

Application-owned classes use provider scope `application`; raw payloads and normalized operational facts retain their actual provider scope. The engine does not yet enforce retention for logs, exports, or backups. Commercial policies remain blocked until FT-301 supplies the provider-contract period and approval reference.

No commercial provider policy may be approved until FT-301 supplies the controlling retention right and approval reference. The shortest applicable provider, operator, legal, or security period must be used.

## Control workflow

All routes require the named administrator-only `ManageRetention` permission and derive the tenant from the verified Rust authorization context.

1. Create a draft policy with `POST /api/admin/retention/policies`. Supply a safe provider identifier, retention seconds, and an opaque approval reference. Retention must be between one hour and ten years.
2. A different administrator approves the policy with `POST /api/admin/retention/policies/{id}/approve`. Approval retires the prior approved policy for the same tenant/data-class/provider scope.
3. Create an inventory with `POST /api/admin/retention/runs/preview`. The run fixes the policy version, provider, cutoff, counts, requester, time, and controlled evidence reference.
4. A different administrator approves the run with `POST /api/admin/retention/runs/{id}/approve`.
5. Execute with `POST /api/admin/retention/runs/{id}/execute`. Execution locks and recounts the fixed scope. It refuses to run if inventory changed or exceeds 10,000 records.
6. Verify the completed run counts, tombstone count, and `retention.run.completed` authorization audit event.

Policy and evidence references accept only bounded identifier characters; they are not free-form note channels.

## Scheduled execution

An administrator may create a schedule for one exact approved policy version with a cadence between one hour and 31 days. A different administrator must activate it. The Rust retention scheduler is a supervised critical worker and turns each due slot into one bounded approved run, using the schedule creator as requester/executor and the schedule approver as the standing second-person approval.

Each attempt records its scheduled slot, preview counts, resulting retention run, completion time, or bounded failure code. Cadence advances from the original slot without drift and a slot cannot execute twice. A new policy version retires the prior active schedule. Inactive/disabled administrators, a retired policy, an oversized scope, or an inventory mismatch pause the schedule and require a new reviewed schedule rather than silently retrying deletion.

## Tombstones and restoration

Every deleted raw payload creates a tenant/provider/feed/SHA-256 tombstone before the payload is cleared. A database trigger applies the tombstone to inserts or raw-payload updates, so restoring or replaying an identical deleted payload keeps it empty and retains the original deletion timestamp/reference.

Authorization-audit, session-revocation, and identity-minimization runs create typed lifecycle tombstones before mutation. Database triggers suppress restored audit/revocation rows and force restored tombstoned identities back to the minimized subject/display-name/disabled state. Shared identities are deliberately excluded until their cross-tenant disposition is approved.

Terminal alert runs create tombstones keyed by alert ID, material dedupe key, and series/revision before deleting actions, evidence, and alerts. Exact replay remains suppressed, while new material evidence continues at the next revision instead of reusing a deleted revision.

Normalized-fact runs create typed provider-scoped tombstones before deleting positions/routes, observations, hazards, and then flights in dependency order. Exact record restoration or replay is suppressed. Active/recent flights, mixed-provider aggregates, any fact referenced by an alert, current facts, and other tenants/providers remain intact.

After any backup restore, restore the current tombstone set from the isolated control copy before allowing application traffic or ingestion. Then verify that a representative tombstoned payload cannot be reintroduced. F401-008 remains open until this is exercised against managed backups with recorded RPO/RTO.

## Integrity verification

An administrator can call `GET /api/admin/retention/integrity` after a restore, retention run, or incident. The tenant-scoped result is healthy only when no current row conflicts with a raw-payload, lifecycle, alert-history, or operational-fact tombstone. It also reports tombstone totals, paused schedules, and failed scheduled attempts from the prior 24 hours as diagnostic context; those operational signals do not by themselves mean deleted data was resurrected.

Follow [`BACKUP_RESTORE_RUNBOOK.md`](BACKUP_RESTORE_RUNBOOK.md) for the controlled restore sequence. Do not reopen traffic until every tenant is healthy, the representative replay probes remain suppressed, and a second reviewer approves the evidence. F401-008 remains open until this procedure is exercised against managed backups with recorded RPO/RTO.

## Failure handling

- `second_administrator_required`: use another active administrator; do not share sessions or change the requester identity.
- `retention_inventory_changed`: abandon the stale run and create a new preview. Do not bypass the recount.
- `retention_scope_too_large`: split the approved scope through a shorter operational batch procedure before code support is expanded; do not raise the bound ad hoc.
- `retention_unavailable`: stop. Confirm database health and preserve the approved preview; do not claim deletion.
- `schedule_authorization_inactive` or `policy_not_approved`: the schedule pauses automatically. Re-establish accountable administrators or approve the replacement policy, then create and separately approve a new schedule.
- `retention_scope_too_large` or `retention_inventory_changed` on a scheduled attempt: investigate the recorded preview, split/remediate the scope, and create a newly approved schedule. Do not resume by editing database state.

Treat partial or unexpected deletion as SEV-2 until scope and recoverability are known. Do not paste payloads, session identifiers, or unrestricted exports into Git or chat evidence.
