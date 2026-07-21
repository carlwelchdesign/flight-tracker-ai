# ADSB.lol Portfolio Integration Decision

Last verified: 2026-07-21

## Decision

Use [ADSB.lol](https://api.adsb.lol/docs) as the optional, zero-data-fee,
best-effort aircraft-position source for the public portfolio demonstration.
Deterministic replay remains the guaranteed demonstration path and the only
fallback. The live source is not required for the site to be usable.

This decision authorizes FT-302 to build an adapter. It does not claim an SLA,
complete coverage, authoritative routes, schedules, flight status, or fitness
for operational use.

## Rights and obligations

ADSB.lol publishes its API and public data under the
[Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
The ODbL grants worldwide, royalty-free use of the database, including public
use and creation of a displayed Produced Work. A public Produced Work must
include attribution. Public use of an adapted database can add share-alike and
machine-readable distribution obligations.

FT-302 must implement these boundaries:

- Fetch server-side through the Rust adapter; the browser never calls the
  provider directly.
- Show `Contains information from ADSB.lol, available under the Open Database
  License (ODbL).` with links to ADSB.lol and the ODbL whenever the live layer is
  visible.
- Treat responses as ephemeral current-state input. Do not persist raw ADSB.lol
  responses or normalized ADSB.lol aircraft positions in PostgreSQL, object
  storage, browser storage, logs, fixtures, analytics, screenshots used as data
  exports, or backups.
- Do not cache provider responses. A sampled API response returned
  `Cache-Control: no-store`; the integration must preserve that policy through
  the Rust API and Next.js boundary.
- Do not offer downloads, exports, historical playback, or redistribution of
  ADSB.lol records. If persistence, enrichment, exports, or a derived database
  are proposed later, stop and review ODbL sections 4.3, 4.4, and 4.6 before
  implementation.
- Keep the live layer outside AI/LLM inputs. The selected use is map display and
  deterministic UI metadata only.
- Re-check the API documentation, license, response headers, and requested
  attribution immediately before public deployment.

ADSB.lol's [API source repository](https://github.com/adsblol/api) describes
dynamic rate limits based on service load and says a future API key may require
feeding data. Its documentation asks production users to make contact so their
usage does not accidentally harm the service. That courtesy contact is
recommended before public launch, but it is not a licensing, procurement, or
portfolio-release gate.

## Bounded access policy

FT-302 must use the regional point endpoint only:

`GET /v2/point/{latitude}/{longitude}/{radius_nm}`

Initial operating limits:

- one configurable demonstration region;
- radius no greater than 100 nautical miles;
- no more than one request in flight;
- refresh no faster than once every 30 seconds;
- five-second request timeout;
- bounded exponential backoff with jitter after transport errors, `429`, or
  `5xx` responses;
- no global scans and no per-aircraft polling; and
- automatic transition to a visible degraded/unavailable state with a direct
  replay action.

The provider publishes no SLA. Dynamic load shedding, rate-limit changes, a
future API-key requirement, incomplete receiver coverage, and individual stale
positions are normal source conditions, not exceptional product failures.

## Canonical mapping boundary

Only documented ADS-B identity, position, motion, and source-quality facts may
cross the adapter boundary. The initial allowlist is:

- transponder identity (`hex`) and callsign (`flight`) when present;
- latitude and longitude;
- geometric or barometric altitude, including the explicit `ground` state;
- ground speed, track, vertical rate, and squawk when present;
- message and position age (`seen`, `seen_pos`); and
- ADS-B source classification needed to explain coverage quality.

The adapter must convert provider units and optional values into the canonical
domain contract and derive observation time from the provider response time and
position age. It must reject impossible coordinates and non-finite values,
preserve missing facts as missing, and avoid leaking provider response types
into the domain or web application.

Undocumented or ambiguous fields are ignored. ADSB.lol is not the source of
truth for origin, destination, route, schedule, delay, cancellation, or
operational status. Scenario routes, schedules, and statuses remain replay data
and must stay visibly labeled `Simulated` when a live position layer is shown.

## Bounded sample evidence

The following read-only inspection was run from one development machine on
2026-07-21 from approximately 17:16–17:17 UTC. No response body was retained.
Counts show only what the crowdsourced receiver network returned in that window.

| Region | Radius | Records | With position | With callsign | Observed position age | Source observations |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| San Francisco (SFO) | 25 nm | 71 | 71 | 68 | 0.018–41.235 seconds | `adsb_icao`, `adsb_other`, and `adsr_icao`; missing callsigns and positions older than 30 seconds occurred |
| New York (JFK) | 25 nm | 110 | 110 | 106 | 0–60.446 seconds | `adsb_icao`; missing callsigns and a position older than one minute occurred |
| London Heathrow (LHR) | 25 nm | 86 | 86 | 81 | 0–39.309 seconds | `adsb_icao`, `adsb_icao_nt`, and `mlat`; missing callsigns occurred |

A follow-up SFO response returned HTTP `200`, `Cache-Control: no-store`, and
31,569 bytes. The records exposed position and motion facts but no authoritative
origin, destination, or route contract. A documentation/OpenAPI request during
the same inspection returned HTTP `503` while regional point requests still
worked. This is direct evidence that the adapter must handle partial service
failure and that replay cannot depend on the live source.

Coverage, completeness, and freshness vary by location, receiver density,
aircraft equipment, source type, and service load. This sample is a schema and
failure-behavior check, not a coverage benchmark.

## Candidates not selected

| Candidate | Outcome | Reason |
| --- | --- | --- |
| ADSB.lol | Selected for an optional position layer | ODbL provides explicit public-use rights and attribution rules; official API documentation exposes a bounded regional endpoint. The project can honor `no-store` through an ephemeral adapter. |
| Airplanes.live | Not selected | Its official guide says non-commercial use, one request per second, and no SLA, but the reviewed public pages did not state an equally precise data license, attribution, retention, or redistribution contract. Do not use it as an automatic secondary live provider. |
| OpenSky | Ineligible without a new agreement | Its default terms require prior written licensing for automated/operational hosted use. That external approval is outside the portfolio scope. |
| Deterministic replay | Required fallback | It is versioned, testable, provider-independent, and available without network or license dependencies. |

## FT-302 activation gate

The live adapter remains disabled until FT-302 proves all of the following:

- provider types terminate at the Rust adapter;
- the allowlist, unit conversions, bounds, duplicate, stale, and out-of-order
  behavior have automated tests;
- responses are not persisted or cached and browser/API headers preserve
  `no-store`;
- one-in-flight polling, timeout, rate limiting, and backoff are enforced;
- source, freshness, coverage quality, attribution, and best-effort state are
  visible;
- simulated route, schedule, and status facts cannot be presented as live; and
- any provider failure leaves deterministic replay usable.

## Revalidation evidence

- [ADSB.lol API documentation](https://api.adsb.lol/docs)
- [ADSB.lol API source and service notes](https://github.com/adsblol/api)
- [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/)
- [Airplanes.live API guide](https://airplanes.live/api-guide/)
- [Airplanes.live data fields](https://airplanes.live/rest-api-adsb-data-field-descriptions/)
- [OpenSky terms of use](https://opensky-network.org/about/terms-of-use)
