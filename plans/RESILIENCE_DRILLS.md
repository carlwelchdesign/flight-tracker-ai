# FT-402 Resilience Drill Contract

Last updated: 2026-07-21

## Purpose and evidence boundary

These drills prove the repository behavior needed for a reliable portfolio
demonstration. They use synthetic replay data, mock provider responses, an
ephemeral PostGIS service, and isolated restore targets. They do not claim a
commercial SLA, lossless provider replay, or a completed managed-host recovery
exercise. FT-404 still owns those deployment-specific gates.

## Automated drill matrix

| Failure | Injection | Required observation | Recovery guarantee |
| --- | --- | --- | --- |
| ADSB.lol outage | Three sanitized provider failures | Source moves through `degraded` to `unavailable`; the UI says replay is preserved and offers `Use replay view` | Replay remains independent and selectable; the source can recover to `current` after a successful poll |
| High provider latency | Mock response exceeds the client timeout | Error code is `timeout`; the first failure is visibly `degraded` | The request ends within the configured bound and does not block replay |
| Malformed/adversarial provider input | Invalid top-level schema, impossible records, duplicates, stale facts, and a response over one MiB | Whole invalid payloads fail closed; bad records are counted as rejected; the freshest valid identity wins | No partial malformed top-level state or oversized body crosses the adapter boundary |
| Alert worker restart | Abort the worker after a human comment, create a new worker, and replay deterministic batches | One alert remains; workflow version and append-only comment are unchanged | PostgreSQL lifecycle history is durable and deterministic replay is idempotent |
| Bounded alert backlog | Queue 208 complete operations-replay batches before the restarted worker begins | A final marker alert proves FIFO drain; elapsed time is printed as `FT402_BACKLOG` | Work below the production 256-batch channel bound drains without duplication or lifecycle loss |
| Receiver overflow | Force a small channel to report skipped batches, then supply a complete newest replay window | Skipped count and recovery time are printed; the final marker alert is created | Skipped unique events are not claimed recoverable; a complete replay/reset window rebuilds current correlation while durable history remains intact |
| Database loss/recovery | Create a controlled marker, take a custom-format dump, restore into an isolated database, and compare release-gate facts | Marker, alert/action counts, migration ledger, and PostGIS extension match; RPO/RTO are printed as `FT402_DATABASE_RECOVERY` | The controlled CI snapshot loses zero committed drill transactions; public traffic remains outside this rehearsal |

## Commands

Provider, parser, UI, and worker contracts:

```sh
cargo test --locked -p flight-tracker-api --lib
npm --prefix apps/web test
```

PostGIS worker/backlog drill:

```sh
TEST_DATABASE_URL='postgres://.../isolated_ft402?sslmode=disable' \
  cargo test --locked -p flight-tracker-api \
  --test resilience_drills -- --nocapture
```

The database recovery script is destructive only to the explicitly named
scratch database ending in `_ft402_restore`. Run it only against an isolated
PostGIS container:

```sh
FT402_POSTGRES_CONTAINER='<isolated-container-id>' \
FT402_ALLOW_DESTRUCTIVE_RECOVERY=true \
  scripts/run_ft402_database_recovery.sh
```

## Findings and operating limits

- Replay fixtures use stable envelope, entity, alert, and dedupe identities.
  Re-emitting them after a worker restart is safe; it does not overwrite or
  duplicate human lifecycle actions.
- Fleet and correlation projections are in-memory current views. Restarting the
  API clears those views until a complete replay window runs again. This is a
  documented portfolio guarantee, not an undisclosed lossless-stream claim.
- Tokio broadcast channels are bounded and deliberately report lag. Work within
  the configured replay capacity drains in order. Once a consumer has lagged,
  skipped unique events are unavailable from that channel; reset/replay is the
  recovery procedure.
- Provider failures do not mark the critical replay worker failed. Database or
  alert-worker failures do fail readiness closed and require repair plus a
  replay reset before the demonstration is considered recovered.
- The CI database rehearsal validates the repository sequence and measures its
  isolated snapshot. Managed backup encryption, access controls, retention,
  regional recovery, approvals, and hosted RPO/RTO remain FT-404 evidence.

## Recorded CI rehearsal

CI run
[29856364366](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29856364366)
recorded the following on its ephemeral Linux/PostGIS environment:

- bounded backlog: 208 batches drained in 1,286 ms with no lifecycle loss or
  duplicate alert;
- forced overflow: 273 batches published, 257 explicitly reported skipped, and
  recovery from the retained complete replay window in 33 ms; and
- database restore: zero controlled transactions lost, 3,520 ms measured RTO,
  PostGIS present, and controlled operator/alert/action/migration counts equal.

These values describe that one controlled CI run. They are not latency targets,
capacity promises, or availability objectives for a hosted service.

## Pass criteria

The ticket passes only when Rust, web, and PostGIS CI are green; CI contains both
`FT402_BACKLOG` measurements and one `FT402_DATABASE_RECOVERY ... result=passed`
line; the recovery target is removed; and the ticket links the exact CI run.
