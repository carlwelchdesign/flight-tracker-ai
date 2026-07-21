# Operations Health and Troubleshooting Runbook

## What the console signals mean

- **Service healthy/degraded** comes from the authenticated Rust `/api/system/health` contract and reflects registered critical workers.
- **Stream live/reconnecting/disconnected** describes the browser's SSE connection. It does not prove source events are fresh.
- **Last event** is the newest provider observation time accepted into the fleet picture.
- **Last received** is the newest time this system accepted that source fact. A widening difference can indicate provider or transport delay.
- **Simulation feed outage** is a deliberate development fault. It suspends replay events while keeping the API, replay runtime, projection worker, and SSE connection alive.
- **Optional live position layer** reports `disabled`, `connecting`, `current`, `degraded`, or `unavailable` from the in-memory Rust ADSB.lol boundary. Provider failure does not disable replay or claim the last accepted positions are current.

Never treat a green connection badge alone as proof that operational data is current.

## First checks

From the repository root, inspect the API directly:

```sh
curl -i http://127.0.0.1:8080/health
curl -i http://127.0.0.1:8080/readiness
curl -i http://127.0.0.1:3001/api/backend/api/system/health
curl -i http://127.0.0.1:3001/api/backend/api/system/readiness
curl -i http://127.0.0.1:3001/api/backend/api/live-positions/status
curl -s http://127.0.0.1:3001/api/backend/metrics
docker compose ps
docker compose logs --tail=100 api
```

Public `/health` should be exactly `{"status":"ok"}`. Public `/readiness` should be `{"status":"ready"}`; a 503 with `not_ready` is expected whenever the database, PostGIS, or a critical worker is unavailable. The authenticated `/api/system/health` response lists `replay_runtime` and `fleet_projection` as `running` in development, and `/api/system/readiness` names the database, PostGIS, and critical-worker checks. The development web BFF commands above create the same short-lived assertion used by the console.

## Correlating a request

Every API response includes `x-correlation-id`. Supply a stable safe ID when reproducing an issue:

```sh
curl -i -H 'x-correlation-id: operator-check-001' http://127.0.0.1:8080/api/flights
docker compose logs api | grep 'operator-check-001'
```

Logs are newline-delimited JSON. Do not put names, credentials, free-form customer text, or other sensitive values into a correlation ID.

## Simulating and restoring a feed outage

Use the console's **Outage** control, or call:

```sh
curl -s -X POST -H 'content-type: application/json' \
  -d '{"active":true}' http://127.0.0.1:8080/api/dev/replay/outage
```

Expected behavior:

1. The console shows a prominent `Simulation feed outage` banner.
2. Last event and last received times stop advancing.
3. The SSE stream can remain live because transport is still available.
4. `/api/system/health` and `/api/system/readiness` remain healthy because no critical worker failed.

Restore through the banner/control, or call the endpoint with `{"active":false}`. Replay reset also restores the feed.

## Failure matrix

| Signal | Likely boundary | First response |
| --- | --- | --- |
| Service degraded; worker `starting` | Worker has not heartbeated | Wait one heartbeat interval, then inspect API logs |
| Service degraded; worker `stale` | Worker task is hung or starved | Capture health and logs, then restart the API process |
| Service degraded; worker `failed` or `stopped` | Critical task exited | Use its JSON log correlation fields, fix the cause, then restart |
| Readiness database unavailable | PostgreSQL/network/credentials | Check database container, connection string, and database logs |
| Readiness PostGIS missing | Wrong database or incomplete migration | Verify the target database and migration state |
| Stream reconnecting with service healthy | Browser-to-API/SSE interruption | Check the proxy route, network, and active-stream metric |
| Stream live but event time stale | Source/provider/replay is not advancing | Check outage state, replay phase, and source timing |
| ADSB.lol degraded or unavailable | Optional regional source timed out, rate-limited, returned invalid data, or failed repeatedly | Keep or select replay, inspect the sanitized error code, confirm the bounded configuration, and retry later; do not increase polling frequency |

## Recovery confirmation

After intervention, require all of the following before considering the development stack recovered:

- `/api/system/health` reports `ok` and every registered worker is `running`.
- `/api/system/readiness` returns HTTP 200 with every check `ok`.
- The console reports service healthy and stream live.
- Last event and last received advance after replay resumes.
- A known correlation ID appears in the JSON request-completion log.

After an alert worker restart or a logged `alert input lagged` event, also reset
and replay the complete deterministic scenario. PostgreSQL retains alert and
append-only action history, and deterministic IDs prevent duplicate lifecycle
records, but the bounded in-memory channel does not recover skipped unique
events. Do not declare recovery until the expected alert count and existing
workflow versions/actions match their pre-failure values.

## Repeatable FT-402 drills

Run the provider/UI tests and PostGIS-backed worker drill described in
[`RESILIENCE_DRILLS.md`](RESILIENCE_DRILLS.md). The worker test prints the
bounded batch count, skipped overflow count, and recovery time. The controlled
database script prints its measured snapshot RPO/RTO and removes only the
validated scratch restore database. These measurements are repository evidence;
they do not replace the hosted recovery and monitoring exercises in FT-404.
