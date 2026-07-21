# M2 — Live Weather and Hazard Intelligence

Default owner: Backend/data engineering, with independent domain review of alert fixtures.

## FT-201 — Ingest and normalize NOAA aviation weather

Status: Complete

Branch: `feat/ft-201-noaa-weather-ingestion`
Final implementation commit: `0949784`
Pull request: [#9](https://github.com/carlwelchdesign/flight-tracker-ai/pull/9)
Owner: Backend/data engineering

Integrate NOAA Aviation Weather data beginning with SIGMETs and METARs while retaining raw evidence.

Dependencies: FT-002, FT-104

Acceptance checklist:

- [x] NOAA client has timeouts, retry policy, backoff, rate discipline, and identifiable user agent.
- [x] Raw payload reference and normalized record are stored transactionally or recoverably.
- [x] Validity windows, issue times, geometry, altitude, and cancellation/update behavior are represented.
- [x] METAR and SIGMET fixtures cover normal, malformed, duplicate, and amended records.
- [x] Source health becomes stale or degraded at documented thresholds.

Verification evidence: `plans/NOAA_INGESTION.md`; normal, malformed, duplicate, amended, and geometry-free cancellation fixtures; focused NOAA client, normalization, source-health, projection, and configuration tests; 47 passing Rust library tests and 5 binary tests; strict workspace Clippy; Rust formatting and release build; 11 passing frontend tests; dependency audit with 0 vulnerabilities; frontend lint, typecheck, and production build; Compose configuration and diff hygiene; implementation commit `0949784`; PR [#9](https://github.com/carlwelchdesign/flight-tracker-ai/pull/9), with Rust, web, and API/PostGIS smoke checks passing, including transactional persistence, revision, cancellation, quarantine, duplicate, and source-health coverage against PostGIS.

## FT-202 — Render weather and hazard layers

Status: Complete

Branch: `feat/ft-202-weather-hazard-layers`
Final implementation commit: `568bd63`
Pull request: [#10](https://github.com/carlwelchdesign/flight-tracker-ai/pull/10)
Owner: Full-stack engineering with product-design review

Display current hazards and airport observations without obscuring the core fleet workflow.

Dependencies: FT-201, FT-103

Acceptance checklist:

- [x] Hazard polygons communicate type, severity, altitude, and validity.
- [x] Layer controls expose timestamp and source.
- [x] Expired or stale data is visually distinct and never silently current.
- [x] Selecting a hazard reveals normalized fields and raw-source access.
- [x] Map performance is measured with a representative regional dataset.

Verification evidence: `plans/WEATHER_LAYERS.md`; 47 passing Rust library tests, 5 binary tests, and the schema contract; strict workspace Clippy; Rust formatting and release build; 15 passing web tests; frontend lint, typecheck, production build, and dependency audit with 0 vulnerabilities; Compose configuration and diff hygiene; deterministic 300-hazard/75-METAR benchmark with a 48.29 ms mean complete render and 0.19 ms mean projection; browser verification at 1180 x 720 and 820 x 900 with keyboard hazard selection, layer visibility, retained-data presentation, responsive layout, no horizontal overflow, and no browser errors; implementation commit `568bd63`; CI correction `225937f`; PR [#10](https://github.com/carlwelchdesign/flight-tracker-ai/pull/10), with Rust, web, and API/PostGIS smoke checks passing, including latest-revision, cancellation, geometry, observation, source-attribution, and raw-source route coverage against PostGIS.

## FT-203 — Implement route–hazard intersection rules

Status: Complete

Branch: `feat/ft-203-route-hazard-rules`
Final implementation commit: `28f227f`
Pull request: [#11](https://github.com/carlwelchdesign/flight-tracker-ai/pull/11)
Owner: Backend/domain engineering with independent fixture review

Create deterministic, versioned rules that consider geometry, time, altitude, direction, and configurable proximity margins.

Dependencies: FT-201, FT-202

Acceptance checklist:

- [x] Great-circle/route geometry and coordinate conventions are tested.
- [x] Rule considers hazard validity and altitude overlap, not polygon intersection alone.
- [x] Alert evidence identifies route version, hazard version, closest approach, and rule version.
- [x] Golden cases cover intersection, near miss, expired hazard, and non-overlapping altitude.
- [x] Rule results are deterministic across replay runs.
- [x] Independent fixture review confirms expected outcomes.

Verification evidence: `plans/ROUTE_HAZARD_RULES.md`; high-latitude great-circle and antimeridian coordinate tests; eight rationale-backed golden cases for intersection, near miss, expiry, cancellation, altitude separation, route direction/progress, and missing-altitude behavior; byte-identical canonical replay batches and serialized decisions across scenario reloads; 54 passing Rust library tests, 5 binary tests, 2 golden/replay contract tests, and the schema contract; strict workspace Clippy; Rust formatting and release build; 15 passing web tests plus lint, typecheck, production build, and dependency audit with 0 vulnerabilities; Compose configuration and diff hygiene; implementation commit `28f227f`; PR [#11](https://github.com/carlwelchdesign/flight-tracker-ai/pull/11), with Rust, web, and API/PostGIS checks passing. The PostGIS 3.5 cross-engine oracle independently confirmed all golden spatial, temporal, altitude, direction, closest-approach, and outcome expectations in [CI run 29809973027](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29809973027).

## FT-204 — Add alert ranking, lifecycle, and deduplication

Status: In progress

Branch: `feat/ft-204-alert-lifecycle`
Final commit: Pending
Pull request: Pending
Owner: Full-stack engineering with dispatcher-workflow review

Turn rule results into a manageable dispatcher queue.

Dependencies: FT-203

Acceptance checklist:

- [ ] Alert severity and attention score are explainable and versioned.
- [ ] Stable dedupe key prevents repeated alerts for the same material condition.
- [ ] New evidence can update or supersede an existing alert without erasing history.
- [ ] Dispatcher can acknowledge, comment, dismiss with reason, and resolve.
- [ ] Every lifecycle change creates an append-only audit event.
- [ ] Queue ordering and suppression behavior have automated tests.

Verification evidence: Pending.
