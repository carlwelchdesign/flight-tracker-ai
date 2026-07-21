# Provider and API Feasibility

Last verified: 2026-07-21

This document is the decision record for flight positions, schedules, aviation
weather, hazards, airport observations, and NOTAM data. The active product scope
is a public, non-commercial portfolio demonstration for recruiters and hiring
managers. Public product pages are not treated as contractual promises. The
commercial research below is retained for a possible future production track;
it is no longer a gate for the portfolio release.

## Decision summary

1. Use the NOAA Aviation Weather Center Data API for MVP METAR, TAF, SIGMET,
   G-AIRMET, and PIREP ingestion. It is public-domain data and technically fit
   for advisory use, but the Internet API has no delivery SLA and must always
   expose freshness and degraded states.
2. Keep deterministic simulation and replay as the only flight-position source
   through M1. Do not add a live position provider merely to make the demo look
   live.
3. Evaluate free, best-effort aircraft-position sources in FT-301 using their
   official terms. A source is eligible only when server-side access and public,
   hosted, non-commercial display are permitted and attribution, rate limits,
   caching, retention, and redistribution rules are implementable.
4. Do not integrate OpenSky into an automated or hosted product without a
   written commercial and operational license. Its default terms do not permit
   this project's intended use.
5. Do not use FlightAware AeroAPI under its published self-service license for
   the dispatcher console. The Premium agreement prohibits commercial aircraft
   situational displays. FlightAware Firehose is the relevant enterprise
   product unless FlightAware grants a written AeroAPI exception.
6. Treat FAA SCDS/SWIFT and NMS as separately gated government integrations.
   Public SCDS access is for non-NAS-impacting, non-operational use and is not a
   sole operational source. The NMS API requires an access request and has no
   public approval-time commitment.

Commercial provider selection is deferred, not blocked work on the active
roadmap. Cirium Sky Stream and FlightAware Firehose remain the researched
production candidates if the project is ever re-scoped for operational use.

## Evaluation context

The active release is a portfolio demonstration that models a dispatcher console
without being used by an airline or charter operator. It may display simulated
aircraft and, if FT-301 confirms eligibility, best-effort live positions. The UI
must identify every source and state that it is not for operational use.

The commercial workload and rights analysis below applies only to a future
business-to-business or operational re-scope.

Cost comparisons use this transparent reference workload unless noted:

- 20 simultaneously monitored flights
- 12 monitored hours per day
- 30 days per month
- one position refresh per minute
- one current position per flight per refresh

This is a planning estimate, not a forecast. Streaming contracts, inactive
aircraft, retries, status calls, schedule lookups, alerts, and historical replay
can materially change cost.

## Capability and commercial matrix

| Source | Intended capability | Rights for this product | Coverage and freshness | Limits and history | SLA and support | Estimated cost | Decision |
| --- | --- | --- | --- | --- | --- | --- | --- |
| NOAA Aviation Weather Center Data API | METAR, TAF, PIREP/AIREP, domestic and international SIGMET, G-AIRMET, airport/station data | NWS information is public domain unless marked otherwise; lawful commercial reuse is allowed without implying endorsement or presenting modified data as official | METAR/TAF/SIGMET are worldwide; G-AIRMET is CONUS. Published cache cadence is 1 minute for METAR/SIGMET/G-AIRMET and 10 minutes for TAF. Observed fixtures confirm timestamps and GeoJSON geometry. | 100 requests/minute maximum, no endpoint more than once/minute per ingestion thread, usually 400 results, 15 days of API history, no browser CORS. Use cache files for full datasets. | No Internet-delivery SLA; NWS says timely delivery is not guaranteed. Status page exists. | No data fee; application infrastructure only. | **Use for MVP**, server-side, with backoff, source timestamps, validity windows, and stale/degraded UI. |
| OpenSky REST API and Trino | ADS-B/Mode S state vectors and research history | Any commercial entity and any operational/automated REST use require a prior written license. Default data rights are non-profit research/education only. Commercial evaluation also requires written permission and has material IP/retention conditions. | Crowdsourced global receiver network with geographic and aircraft gaps. State-vector resolution is 10 seconds anonymous and 5 seconds authenticated; individual position timestamps can be absent or stale. OpenSky explicitly has no live schedules, delays, or cancellations. | Anonymous 400, standard 4,000, feeder 8,000 daily credits per endpoint bucket; licensed 14,400 hourly. Authenticated state history is one hour; full history is primarily research Trino/curated data. Anonymous quotas are IP-bucketed. | Provided as-is; no accuracy, timeliness, completeness, reliability, availability, or fixed access-policy warranty. | Personal/non-profit tiers have no published fee; commercial license price is **quote required**. | **Do not integrate** until written rights cover automated cloud hosting, commercial use, retention, derived works, and product IP. Not a schedule source. |
| FlightAware AeroAPI Premium | Pull-based current/historical flight status, positions, schedules, tracks, alerts | Premium permits B2B derivative works, but the published Premium license expressly prohibits using AeroAPI data for commercial aircraft situational displays. Raw data retention is limited to 30 days; combining it as backfill with other real-time providers requires written permission. | FlightAware markets global terrestrial and optional Aireon coverage. Historical status/tracks are available from 2011. Poll freshness depends on endpoint and client cadence. | Premium: 100 result sets/second and 500,000 historical result sets/month. One result set is up to 15 records. | Premium publishes a 99.5% uptime guarantee plus email and phone support. | $1,000/month minimum plus query fees. Reference workload is about $2,160/month list price using fleet search positions (two $0.05 result sets/minute), or $4,320 using individual $0.01 position calls, before published volume discounts and other calls. | **License mismatch** for this UI. Use only with a negotiated written exception; otherwise evaluate Firehose. |
| FlightAware Firehose | Enterprise streaming flight positions, status, plans, schedules, surface movement, replay, optional Aireon/datalink/radar | Product page explicitly lists aircraft situational displays and airline-owned operational tools as use cases. The July 2026 standard terms restrict use to authorized parties described in the Order, prohibit redistribution unless expressly allowed, default data retention to 24 hours unless the Order permits longer, and require deletion at termination. They also require FlightAware's prior written authorization before provider data is disclosed, transferred, processed, uploaded to, or used with any broadly defined AI System. Aireon and other upstream data carry additional SaaS/third-party restrictions. Exact display, tenant, AI/ML, retention, combination, LADD/PIA, derived-data, and customer-data rights therefore remain Order-specific. | Terrestrial ADS-B worldwide plus optional Aireon, datalink, radar, MLAT, and surface layers. Per-aircraft update interval and enabled layers are contract-specific. Point-in-time recovery and historical replay are supported. | Persistent TCP/TLS 1.2+ stream on port 1501; four connections/account by default; filters, data layers, position cadence, history start, and replay access are Order-specific. Current protocol is version 37; non-latest major releases are supported for up to one year. The self-service trial is limited historical data; a comparable real-time trial requires sales provisioning. | 24x7 phone support, redundancy, and high-SLA options are advertised; binding percentage, exclusions, credits, and remedies are **quote required**. | Fixed monthly fee varies by region/operator scope, layers, repurposing, and redistribution; proposal is **quote required**. | **Commercial finalist with a critical AI-rights gate**. Request a matched 14-day real-time target-tail trial and a controlling Order that expressly authorizes the intended SaaS, retention, and AI processing. |
| Cirium Sky API and Sky Stream | Integrated status, positions, schedules, plans, NOTAMs, weather, alerts, and operational stream | Cirium explicitly positions Sky Stream for real-time situational awareness and operational coordination. Exact B2B display, tenant, redistribution, storage, source-combination, blocked-aircraft, derived-data, and customer-data rights remain contractual. | Cirium markets fused flight status, aircraft identity, and positional data across pre-, tactical, and post-flight phases. Raw-source availability, update cadence, target-tail coverage, and quality flags must be measured and contracted. | Sky Stream is a push feed over AMQP 0.9 with GeoJSON messages and offers a 14-day trial to new users. Sky API has separate evaluation plans and limits; those REST trial entitlements must not be assumed to cover Sky Stream production behavior. | API-plan support is published; no binding Sky Stream uptime percentage, credits, or incident targets were found publicly. Contract SLA is **quote required**. | Sky Stream/contract price is **quote required**. | **Commercial finalist**. Trial Sky Stream against the same tails, regions, window, and event-time metrics as Firehose. |
| FAA SWIM via SCDS/SWIFT | Near-real-time U.S. NAS weather, flight/flow, aeronautical, and surveillance messages through Solace JMS | Public SCDS data is pre-approved for release, but SCDS is intended for non-NAS-impacting, non-operational use and should not be the sole source for aviation-safety activity. Airline operational situational awareness is identified as NAS-impacting. Service-specific agreements and LADD obligations apply. | U.S. NAS services; public datasets vary. SCDS is near-real-time but service-specific timing must come from each service description. Some CDM, TFMS request/reply, international, sensitive, and web-service access require separate review or NESG. | Account/subscription approval is automatic after the applicable Service Access Agreement, which renews annually. JMS, not REST. Inactive subscriptions/accounts can be disabled. Standard high-volume threshold is 200 GB/day or 2 TB/month before redistribution/exemption requirements. | Portal reports Up/Degraded/Down/Maintenance and history. No public operational SLA; SCDS is not intended as the sole operational source. | FAA currently charges no data fee; consumer bears interface, operations, and any NESG costs. | **Research/secondary source only** until FAA confirms the permitted path for this operator-facing use. |
| FAA NMS API / FNS NDS | U.S. legacy and digital NOTAM distribution, with near-real-time modernization | Registration, a service agreement, service-specific terms, redistribution restrictions, and LADD compliance apply. New NMS API documentation is access-controlled and must be requested. | U.S. NOTAM source, including legacy and digital data. FAA pages show an active modernization while public transition dates remain incomplete; ICAO-format transition date is not announced. | Legacy FNS NDS uses JMS through SCDS; new NMS is an API. Public quota, history, retention, and complete transition details were not found. | No public API SLA or access-approval target found. | No public API fee found; implementation and onboarding cost remain. | **Access-gated**. Submit the NMS API request now; do not schedule production integration until credentials, schema, terms, coverage, and transition status are verified. |

## OpenSky hosting and operational restrictions

There is no official statement in the reviewed sources that names or bans a
specific cloud host. The actual hosting constraints are more fundamental:

- a live service, automated system, or internal operational integration needs a
  written agreement regardless of the organization's profit status;
- all commercial-entity use needs a written license;
- anonymous limits are bucketed by public IP, so shared NAT or horizontally
  scaled egress does not create independent entitlement and may create
  unpredictable contention;
- credentials and licensed data must remain server-side and cannot be shared
  outside authorized collaborators;
- access policies can change and access can be revoked;
- default commercial-evaluation terms include data deletion and IP provisions
  that require explicit written exceptions before product development.

Therefore “it works from a cloud server” is not evidence that hosted use is
permitted. No OpenSky data or fixture should be committed to this repository
until the written license covers that act.

## NOAA ingestion contract for M2

The NOAA integration should use these server-side endpoints:

| Product | Endpoint | Poll/cache expectation | Required canonical fields |
| --- | --- | --- | --- |
| METAR | `/api/data/metar` | Narrow airport queries no more than once/minute; current full cache updates once/minute | station, raw observation, report/receipt times, coordinates, wind, visibility, ceiling/category |
| TAF | `/api/data/taf` | Current full cache updates every 10 minutes | station, issue time, validity range, forecast periods, raw text |
| Domestic SIGMET | `/api/data/airsigmet` | GeoJSON or current cache; cache updates once/minute | series ID, hazard, issue/valid times, altitude band, geometry, raw text |
| International SIGMET | `/api/data/isigmet` | Narrow query; validate regional coverage during FT-201 | FIR, hazard, issue/valid times, altitude band, geometry, raw text |
| G-AIRMET | `/api/data/gairmet` | GeoJSON or current cache; cache updates once/minute | product/tag, hazard, issue/valid time, forecast hour, level, geometry |
| PIREP/AIREP | `/api/data/pirep` | Narrow spatial/time query or current cache | report/receipt times, aircraft report type, altitude, location, weather fields, raw text |

Implementation constraints:

- Send a descriptive user agent and use bounded exponential backoff with jitter.
- Treat HTTP 204 as a successful query with no current data, not a provider
  outage; handle 429 and 5xx as explicit source-health events.
- Store raw payload references separately from normalized observations.
- Preserve report, receipt, provider-fetch, and processed times.
- Never infer freshness only from request success. Compare product timestamps and
  validity windows to the application clock.
- Do not call this API from the browser; CORS is not supported and credentials,
  throttling, caching, and provenance belong in the Rust ingestion boundary.
- Candidate transport-health thresholds for FT-201 are warning after two missed
  expected polls and degraded after three; product-age thresholds must be defined
  separately with domain review.

Observed payloads and hashes are recorded in
[`evidence/ft-003/NOAA_API_FIXTURES.md`](evidence/ft-003/NOAA_API_FIXTURES.md).

## Optional future commercial evaluation gate

Before any future ticket selects Cirium Sky Stream or FlightAware Firehose for
commercial or operational use, obtain written answers and contract language for
every item below. These requirements do not apply to the active portfolio roadmap.

Use the executable procurement package in [`provider-evaluation/`](provider-evaluation/README.md). It defines the common questionnaire, evidence register, paired trial protocol, scorecard, cost model, decision gates, and structural validator. Confidential contracts, credentials, target tails, quotes, and raw licensed payloads remain outside Git.

### Rights and governance

- B2B use in an airline/charter aircraft situational and operations display
- display to each tenant/operator and any downstream redistribution
- raw retention, normalized-fact retention, replay fixtures, derived works, and
  deletion after termination
- use of provider data, normalized facts, evidence, screenshots, or outputs with
  any LLM, machine-learning model, or other contract-defined AI system,
  including approved vendors, fields, purposes, retention, and training limits
- combination, comparison, and backfill with NOAA, FAA, operator, ADS-B, or a
  second commercial source
- upstream-source permissions, including multi-tenant SaaS processing and
  authorized-user display of Aireon or other third-party data
- LADD, PIA, blocked-aircraft, privacy, export-control, and audit obligations
- attribution, provider branding, and whether provider identity can be exposed
  as alert evidence

### Technical and operational fit

- airborne, surface, oceanic, polar, and target regional/tail coverage
- p50/p95/p99 source-to-delivery latency and position update intervals
- source flags for observed, fused, estimated, and derived positions
- flight identity continuity across call-sign, tail, diversion, cancellation,
  codeshare, wet lease, and day boundaries
- schedule horizon, status history, route/flight-plan availability, replay, and
  point-in-time recovery
- quotas, burst behavior, reconnect/replay rules, maintenance notifications,
  version support, sandbox parity, and incident history
- uptime definition, exclusions, service credits, support response times, data
  correction workflow, security evidence, and breach notification

### Trial scorecard

Run both finalists for at least 14 calendar days against the same operator-owned
tail set and routes. Record:

- percentage of expected flights identified correctly
- percentage of flight time with a position younger than 15, 30, and 60 seconds
- p50/p95/p99 provider receipt lag and longest position gap by region
- schedule, tail, origin/destination, route, diversion, and cancellation accuracy
- number and duration of feed disconnects; replay recovery completeness
- monthly cost at 20, 100, and 500 simultaneously monitored flights
- unresolved contract exceptions and implementation effort

No provider wins solely on marketing coverage or the lowest estimate. Rights for
the exact display and measured target-fleet performance are pass/fail gates.

## FAA access and lead-time plan

| Access path | Public onboarding evidence | Lead-time conclusion | Project action |
| --- | --- | --- | --- |
| SCDS/SWIFT standard public subscription | Register, sign the service-specific SAA, subscribe; account and subscription approval are described as automatic. SAA renews annually. | Potentially same-day after agreement, but provisioning/connectivity time is not guaranteed. | May be evaluated for non-operational research only. Confirm use classification with FAA before consuming data in this product. |
| SCDS sensitive/CDM or unsupported TFMS services | Sensitive data requires NDRB approval; CDM/TFMS request/reply/international paths require direct FAA request and policy review. | **Unknown and manually reviewed**; no public target found. | Treat as an external dependency with no committed delivery date. Start inquiry before M3 if needed. |
| New NMS API | FAA page requires a direct API-access request; public documentation is not available before access. | **Unknown**; no public approval SLA found. | Submit request during M0. No future NOTAM integration ticket may commit delivery dates until access, schema, terms, and transition status are confirmed. |
| Legacy FNS NDS via SCDS | FAA Agreement Portal registration, service agreement, JMS subscription, and service description apply. | Standard SCDS approval may be automatic, but technical onboarding and modernization migration remain uncertain. | Use only as an evaluated fallback; avoid building a new long-lived dependency on a service being replaced. |

## Source register

All sources below are provider or government primary sources and were checked on
2026-07-21.

### NOAA/NWS

- [Aviation Weather Center Data API](https://aviationweather.gov/data/api/)
- [Aviation Weather Center service status](https://aviationweather.gov/tools/status/)
- [NWS disclaimer and public-domain use](https://www.weather.gov/disclaimer)

### OpenSky

- [General Terms of Use and Data License](https://opensky-network.org/about/terms-of-use)
- [REST API documentation and credits](https://openskynetwork.github.io/opensky-api/rest.html)
- [OpenSky FAQ](https://opensky-network.org/about/faq)
- [OpenSky data access overview](https://opensky-network.org/data/)

### FlightAware

- [AeroAPI tiers, rights, limits, SLA, and query prices](https://www.flightaware.com/commercial/aeroapi/)
- [AeroAPI Premium License Agreement, March 2025](https://www.flightaware.com/commercial/aeroapi/AeroAPI_Premium_License_Mar2025.pdf)
- [AeroAPI history and billing FAQ](https://www.flightaware.com/commercial/aeroapi/faq.rvt)
- [Firehose product and operational use cases](https://www.flightaware.com/commercial/firehose/)
- [Firehose protocol](https://www.flightaware.com/commercial/firehose/documentation/)
- [Firehose initiation, filtering, and update intervals](https://www.flightaware.com/commercial/firehose/documentation/commands)
- [Firehose connection, reconnect, and connection-count guidance](https://www.flightaware.com/commercial/firehose/documentation/connection)
- [Firehose product FAQ, real-time trial path, support, SLA options, and pricing factors](https://support.flightaware.com/hc/en-us/articles/37737289228311-Firehose-Product-FAQ)
- [FlightAware data-service terms, July 2026](https://www.flightaware.com/commercial/termsandconditions)

### Cirium

- [Cirium aviation data overview](https://developer.cirium.com/apis/data/overview)
- [Cirium Sky API subscriptions and trial limits](https://developer.cirium.com/apis/cirium-sky-api/subscriptions)
- [Current Flight Tracks API](https://developer.cirium.com/apis/cirium-sky-api/flight-track)
- [Flight Schedules API](https://developer.cirium.com/apis/cirium-sky-api/schedules)
- [Cirium Developer Studio and Sky Stream](https://www.cirium.com/data/aviation-api/)
- [Cirium Sky Stream overview](https://developer.cirium.com/apis/cirium-sky-stream/overview)
- [Cirium Sky Stream onboarding, 14-day trial, AMQP, and GeoJSON](https://developer.cirium.com/apis/cirium-sky-stream/get-started)
- [Cirium schedules coverage](https://www.cirium.com/data/flight-schedules/schedules-and-connections-data/)

### FAA

- [Getting access to SWIM](https://www.faa.gov/air_traffic/technology/swim/products/get_connected)
- [SWIM questions, data cost, and SCDS use](https://www.faa.gov/air_traffic/technology/swim/questions_answers)
- [SCDS General Guideline and Standards, version 1.1](https://www.faa.gov/sites/faa.gov/files/air_traffic/technology/swim/governance/SCDS-Guideline-Document_v1.1_09.11.2024)
- [SCDS approval timing](https://support.swim.faa.gov/hc/en-us/articles/360034504091-How-long-is-the-approval-process-to-receive-the-desired-data-set-from-sending-the-request-until-receiving-access)
- [SCDS Service Access Agreement renewal](https://support.swim.faa.gov/hc/en-us/articles/24963265968148-Service-Access-Agreement)
- [FAA Data Agreement Portal and FNS NDS terms](https://aa.data.faa.gov/data/service.jsf?uuid=08c4033e-5faf-421f-b235-71f28ca5d8d9)
- [NOTAM Management Service and API access](https://www.faa.gov/about/initiatives/notam)
- [NMS API and transition FAQ](https://www.faa.gov/about/initiatives/notam/faqs)

## Revalidation rule

Provider pages, licenses, prices, quotas, and government transition plans can
change. Re-check the source register before opening any provider account, starting
an integration ticket, signing a contract, or using these estimates in a budget.
