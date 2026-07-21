# Comparable 14-Day Real-Time Trial Protocol

## Objective

Measure whether Cirium Sky Stream and FlightAware Firehose can lawfully and reliably support the same target operator workflow. This is a paired operational evaluation, not a demo of whichever sample data a provider makes easiest to access.

## Preconditions

Do not start the trial clock until all conditions are true:

- both providers confirm real-time access for the same 14-calendar-day UTC window;
- the same authorized target tails, expected flights, regions, and phases are in scope;
- the proposed production data layers and quality/source flags are enabled;
- test retention, deletion, confidentiality, and result-sharing terms are accepted in writing;
- both endpoints are observed from the same cloud region and synchronized clock;
- credentials are stored outside Git and log redaction is verified;
- a dry run proves raw capture, reconnect, clock offset, and missing-value behavior.

If one provider cannot begin within the agreed window, pause rather than compare different operating periods unless Product and Engineering document why seasonality, weather, route mix, and schedule changes cannot bias the result.

## Frozen population

The controlled target manifest must contain an opaque `flight_key`, authorized aircraft registration, expected callsign, scheduled origin/destination, scheduled off/on times, operating carrier, region, and authorization reference. Commit only a redacted summary containing counts by region and phase.

Minimum strata should include, when the operator actually flies them:

- continental domestic;
- oceanic or remote;
- polar/high-latitude;
- high-density terminal and surface;
- diversion, cancellation, codeshare, wet-lease, or tail-swap cases observed during the window.

Do not invent flights to fill a stratum. Record `not_observed` and the resulting uncertainty.

## Observation contract

Normalize each provider delivery into an append-only trial observation with:

| Field | Definition |
| --- | --- |
| `provider` | `cirium_sky_stream` or `flightaware_firehose`. |
| `flight_key` | Trial-local opaque identifier mapped to the controlled manifest. |
| `region` | Frozen region taxonomy from the manifest. |
| `message_type` | Position, schedule, status, route, cancellation, diversion, connection, or replay. |
| `provider_event_id` | Provider identifier retained only when trial terms permit it. |
| `source_event_at` | Provider-declared fact or observation time, nullable when absent. |
| `provider_sent_at` | Provider delivery timestamp, nullable when absent. |
| `received_at` | Collector receipt time from a synchronized UTC clock. |
| `processed_at` | Normalization completion time. |
| `source_quality` | Observed, fused, estimated, derived, unknown, or provider-specific unmapped. |
| `is_replay` | Whether the record arrived through reconnect/PITR replay. |
| `connection_id` | Trial-local connection attempt identifier. |
| `missing_fields` | Explicit field names; never replace missing with zero. |

Provider payloads remain behind provider-specific adapters. The normalized trial facts may share a schema, but adapters must retain enough provenance to audit a mismatch.

## Metric definitions

Metrics are computed separately by provider and region, with global totals reported only as a roll-up.

| Metric | Grain and formula | Null/missing rule |
| --- | --- | --- |
| Expected-flight identification | Expected manifest flight; correctly correlated flights / eligible expected flights. | Missing provider flight is a failure; operator-cancelled flight remains eligible for cancellation accuracy. |
| Position availability | One-second flight-time interval; intervals covered by at least one position / expected airborne intervals. | Unknown truth interval is excluded with reason, never counted as success. |
| Position age ≤15/30/60s | Delivered position; `received_at - source_event_at` at each threshold. | Missing source event time is `unknown_age` and reported separately. |
| Delivery lag p50/p95/p99 | Delivered record; percentile of `received_at - provider_sent_at`, or source event time when sent time is unavailable with a labeled method. | Negative or impossible lag is quarantined as clock/data error. |
| Longest position gap | Flight and region; maximum interval between consecutive source observation times while expected airborne. | Region boundary method and gaps crossing it must be documented. |
| Schedule accuracy | Expected flight field; exact normalized match for date, carrier, number, origin, destination, and scheduled times within agreed tolerance. | Report each field separately; no composite success when a required field is missing. |
| Tail continuity | Expected flight; correct aircraft identity across swaps and day boundaries. | Unknown truth is excluded and counted in uncertainty. |
| Diversion/cancellation accuracy | Eligible event; true positive, false positive, false negative, and detection lag. | Report confusion matrix, not accuracy alone. |
| Disconnect count/duration | Connection attempt; provider or network disconnects and wall-clock unavailable duration. | Planned client restarts are labeled separately. |
| Replay completeness | Reconnect window; unique expected records recovered / records known missing at disconnect. | Duplicate replay records are reported separately and do not increase completeness. |
| Processing cost | Scenario/month; contracted fixed plus usage, environment, connection, replay, support, and overage charges. | Unpriced component makes total `incomplete`, not zero. |

Percentiles use the nearest-rank method over valid nonnegative observations. Every metric row records numerator, denominator, sample count, unit, trial-window bounds, method version, and evidence reference.

## Failure and bias controls

- Maintain collector clock offset under 100 ms; quarantine affected intervals when that cannot be proven.
- Separate provider delivery lag from application processing lag.
- Preserve duplicates and out-of-order arrivals as quality measurements before deduplication.
- Record collector, network, credential, quota, planned maintenance, provider, and unknown outages separately.
- Do not exclude a bad region, aircraft, or day after results are visible.
- Do not tune provider filters differently unless the proposed production packages require it; record every difference.
- Report both raw counts and percentages so small strata remain visible.
- Freeze metric version `ft301-v1` before the first production trial record.

## Trial operations

1. Run a 60-minute paired shakedown that is excluded from the scored window.
2. Start both scored collectors at the same UTC instant.
3. Review capture health twice daily without changing filters or dropping poor data.
4. Exercise one controlled reconnect per provider after day two and one recovery during a naturally occurring disconnect when available.
5. End capture after 14 complete calendar days.
6. Export aggregates, validate the scorecard, and delete or quarantine raw data according to the stricter trial term.
7. Have Engineering sign the technical result and Legal confirm that retained evidence is permitted.

## Exit criteria

The trial is comparable only when both providers have at least 14 complete days, the same eligible population, collector availability of at least 99.9% excluding recorded provider outages, and no unresolved metric-integrity defect. Otherwise extend or rerun; do not score an invalid window.
