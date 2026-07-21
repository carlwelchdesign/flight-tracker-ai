# Fleet API and Live Event Contract

FT-102 projects normalized ingestion batches into current flight state, per-flight history, and a retained live-event stream. The projection is independent of Axum and provider adapters: replay and future live providers enter through the same `NormalizedEventBatch` boundary.

## Read endpoints

### `GET /api/flights`

Query parameters:

- `page`: one-based page number; default `1`.
- `page_size`: items per page from `1` through `100`; default `25`.

The response contains `data`, an array of typed flight views with canonical flight data and the latest accepted position, plus `pagination` with `page`, `page_size`, `total_items`, and `total_pages`. Results sort by callsign and then flight UUID for stable pagination.

### `GET /api/flights/{flight_id}`

Returns one typed flight view. A malformed UUID returns `400 invalid_flight_id`; an unknown valid UUID returns `404 flight_not_found`.

### `GET /api/flights/{flight_id}/timeline`

Uses the same pagination parameters and metadata. Events are ordered by event time and then monotonic event ID. Each external fact retains its provider envelope ID and complete source attribution.

All API errors use `{ "error": { "code": "...", "message": "..." } }`.

## Live stream

`GET /api/events/stream` returns `text/event-stream`. Each message has:

- SSE event type `fleet_event`.
- A monotonically increasing integer SSE `id`.
- JSON data containing the event ID, optional flight ID, provider envelope ID, event time, optional source attribution, and canonical event.

Clients reconnect with the standard `Last-Event-ID` header. The service first replays retained events with higher IDs, subscribes before taking the retained snapshot to avoid a handoff gap, filters duplicates at the stream boundary, and then continues with live events. A malformed header returns `400 invalid_last_event_id`. The server sends a keep-alive comment every 15 seconds.

## Projection invariants

- A provider envelope is applied at most once until a development replay reset.
- An empty batch, tenant mismatch, or source-attribution mismatch rejects the whole batch.
- A flight or position older than or equal to the accepted current fact is ignored.
- A position for an unknown flight is ignored rather than creating partial flight state.
- Rejected, duplicate, orphaned, and out-of-order facts cannot replace current state or enter a flight timeline.
- Replay reset clears current state, timelines, retained events, and envelope deduplication. The next SSE ID remains monotonic so connected clients never confuse a reset replay with old messages.

The FT-102 projection is deliberately in-memory for the deterministic M1 console. Database-backed recovery and production retention belong to the commercial-data and operational-hardening milestones.

## Metrics

Every API request receives an `x-correlation-id`. A caller-supplied ID is preserved only when it is a safe 1–128 character identifier; otherwise the API creates a UUID. The response repeats the ID. JSON tracing events include correlation ID, method, path, status, and latency in microseconds. Projection logs use the provider-envelope UUID as their correlation ID, and replay lifecycle logs use the scenario ID. SSE connection open/close transitions emit the active connection count. `GET /metrics` exposes Prometheus text counters:

- `flight_tracker_api_requests_total`
- `flight_tracker_api_latency_microseconds_total`
- `flight_tracker_stream_connections_total`
- `flight_tracker_stream_connections_active`
