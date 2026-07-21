# Operations Health and Troubleshooting Runbook

## What the console signals mean

- **Service healthy/degraded** comes from the Rust `/health` contract and reflects registered critical workers.
- **Stream live/reconnecting/disconnected** describes the browser's SSE connection. It does not prove source events are fresh.
- **Last event** is the newest provider observation time accepted into the fleet picture.
- **Last received** is the newest time this system accepted that source fact. A widening difference can indicate provider or transport delay.
- **Simulation feed outage** is a deliberate development fault. It suspends replay events while keeping the API, replay runtime, projection worker, and SSE connection alive.

Never treat a green connection badge alone as proof that operational data is current.

## First checks

From the repository root, inspect the API directly:

```sh
curl -i http://127.0.0.1:8080/health
curl -i http://127.0.0.1:8080/readiness
curl -s http://127.0.0.1:8080/metrics
docker compose ps
docker compose logs --tail=100 api
```

`/health` should be HTTP 200 and list `replay_runtime` and `fleet_projection` as `running` in development. `/readiness` should be HTTP 200 with database, PostGIS, and critical workers all `ok`. A 503 readiness response is expected whenever any one of those dependencies is unavailable.

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
4. `/health` and `/readiness` remain healthy because no critical worker failed.

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

## Recovery confirmation

After intervention, require all of the following before considering the development stack recovered:

- `/health` reports `ok` and every registered worker is `running`.
- `/readiness` returns HTTP 200 with every check `ok`.
- The console reports service healthy and stream live.
- Last event and last received advance after replay resumes.
- A known correlation ID appears in the JSON request-completion log.
