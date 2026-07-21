# NOAA Aviation Weather Ingestion

FT-201 adds server-side NOAA Aviation Weather Center ingestion for selected-airport METARs and current domestic SIGMETs. NOAA types stop at the adapter boundary; downstream consumers receive provider-independent `AirportObservation`, `WeatherHazard`, and `SourceHealth` records.

## Provider contract

- Base service: `https://aviationweather.gov/api/data/`
- METAR request: `GET /metar?ids=<ICAO list>&format=json&hours=2`
- Domestic SIGMET request: `GET /airsigmet?format=json`
- Each endpoint is polled no more than once per minute. Configuration rejects a shorter interval.
- Requests use bounded connect/request timeouts, an identifiable user agent, and up to three attempts with bounded exponential backoff and full jitter.
- HTTP 204 means a successful request with no current product. HTTP 429 and retryable 5xx/network failures affect source health and are retried within the request budget.
- The browser never calls NOAA directly.

The contract was revalidated against the AviationWeather.gov v4 OpenAPI schema and live, narrowly scoped METAR/SIGMET responses on 2026-07-20.

## Configuration

NOAA ingestion is opt-in:

| Variable | Meaning | Default |
| --- | --- | --- |
| `ENABLE_NOAA_WEATHER` | Enables the two polling loops | `false` |
| `NOAA_OPERATOR_ID` | Existing tenant/operator UUID that owns ingested records | none when disabled |
| `NOAA_METAR_STATIONS` | Comma-separated four-character ICAO identifiers | none when disabled |
| `NOAA_POLL_INTERVAL_SECONDS` | Interval per endpoint; minimum 60 | `60` |
| `NOAA_USER_AGENT` | Identifiable HTTP user agent | repository URL identifier |
| `NOAA_API_BASE_URL` | Provider base URL; primarily a test seam | `https://aviationweather.gov/` |

Startup fails before polling if the configured operator does not exist. Provider credentials are not required and no NOAA settings are exposed to the web client.

## Persistence and revisions

Each provider record becomes a `provider_envelopes` row with immutable source identity and SHA-256 evidence. Raw JSON remains available only until an approved retention run clears it and attaches a restore-suppression tombstone; normalized record identity is preserved. The matching normalized record is written in the same transaction:

- METARs become `airport_observations` with report time, NOAA receipt time, local receipt/processing time, WGS84 point, wind in knots/true degrees, visibility in statute miles, ceiling in feet AGL, and flight category.
- SIGMETs become versioned `weather_hazards` with issuance and validity times, NOAA receipt time, WGS84 polygon, flight-level altitude band, hazard/severity, stable external series identity, revision, superseded revision, and active/cancelled status.
- A cancellation without its own geometry explicitly supersedes the prior series revision and carries that prior footprint forward as cancellation evidence.
- Identical provider record/hash deliveries are idempotent. A changed payload with the same provider identity is retained as a new envelope and SIGMET revision.
- Records that cannot be normalized keep their raw envelope with `processed_at = null` and an `ingestion_failures` quarantine record. They are not published as canonical events.

Committed normalized events publish through the same ingestion/projection boundary used by replay. A provider outage does not crash the critical worker or erase the last accepted picture.

## Source-health policy

`GET /api/source-health` returns persisted health for `metar` and `airsigmet`, including last attempt/success, newest source event, delay, consecutive failures, threshold, and last error code.

- Successful transport with current products: `healthy`.
- Successful HTTP 204/no active SIGMET: `healthy`; absence is not an outage.
- METAR product age over 15 minutes: `stale`, even if HTTP requests succeed.
- SIGMET transport has a 3-minute stale threshold; active validity is evaluated on each hazard record rather than treating an empty valid set as stale.
- First consecutive failed poll: `unknown` while retry/recovery evidence is incomplete.
- Second consecutive failed poll: `stale` (two missed expected polls).
- Third and later consecutive failed polls: `degraded`.
- Any successful poll resets the transport-failure count. A quarantined record counts as a failed feed result because successful HTTP transport is not proof of usable data.

## Operations and recovery

1. Check authenticated `/api/system/health` for `noaa_weather_ingestion` and `noaa_projection`. These workers should remain running during provider failures.
2. Check `/api/source-health` for the affected feed, timestamps, failure count, and error code.
3. For `rate_limited`, confirm the interval is at least 60 seconds and only one deployment is polling for this operator/feed.
4. For `timeout` or `provider_unavailable`, check the AviationWeather.gov status page and outbound network path.
5. For `record_quarantined`, inspect `ingestion_failures` and its source envelope; do not discard or hand-edit the raw evidence.
6. Recovery is proven by a successful poll, a reset failure count, advancing source timestamps when a product exists, and newly committed normalized records.

This feed is advisory evidence. Product validity and freshness remain visible and must not be presented as certified flight-planning authority.

## Primary references

- [AviationWeather.gov Data API](https://aviationweather.gov/data/api/)
- [AviationWeather.gov OpenAPI schema](https://aviationweather.gov/data/schema/openapi.yaml)
- [AviationWeather.gov service status](https://aviationweather.gov/tools/status/)
