# M2 — Live Weather and Hazard Intelligence

Default owner: Backend/data engineering, with independent domain review of alert fixtures.

## FT-201 — Ingest and normalize NOAA aviation weather

Status: Not started

Branch: `feat/ft-201-noaa-weather-ingestion`
Final commit: Pending
Pull request: Pending

Integrate NOAA Aviation Weather data beginning with SIGMETs and METARs while retaining raw evidence.

Dependencies: FT-002, FT-104

Acceptance checklist:

- [ ] NOAA client has timeouts, retry policy, backoff, rate discipline, and identifiable user agent.
- [ ] Raw payload reference and normalized record are stored transactionally or recoverably.
- [ ] Validity windows, issue times, geometry, altitude, and cancellation/update behavior are represented.
- [ ] METAR and SIGMET fixtures cover normal, malformed, duplicate, and amended records.
- [ ] Source health becomes stale or degraded at documented thresholds.

Verification evidence: Pending.

## FT-202 — Render weather and hazard layers

Status: Not started

Branch: `feat/ft-202-weather-hazard-layers`
Final commit: Pending
Pull request: Pending

Display current hazards and airport observations without obscuring the core fleet workflow.

Dependencies: FT-201, FT-103

Acceptance checklist:

- [ ] Hazard polygons communicate type, severity, altitude, and validity.
- [ ] Layer controls expose timestamp and source.
- [ ] Expired or stale data is visually distinct and never silently current.
- [ ] Selecting a hazard reveals normalized fields and raw-source access.
- [ ] Map performance is measured with a representative regional dataset.

Verification evidence: Pending.

## FT-203 — Implement route–hazard intersection rules

Status: Not started

Branch: `feat/ft-203-route-hazard-rules`
Final commit: Pending
Pull request: Pending

Create deterministic, versioned rules that consider geometry, time, altitude, direction, and configurable proximity margins.

Dependencies: FT-201, FT-202

Acceptance checklist:

- [ ] Great-circle/route geometry and coordinate conventions are tested.
- [ ] Rule considers hazard validity and altitude overlap, not polygon intersection alone.
- [ ] Alert evidence identifies route version, hazard version, closest approach, and rule version.
- [ ] Golden cases cover intersection, near miss, expired hazard, and non-overlapping altitude.
- [ ] Rule results are deterministic across replay runs.
- [ ] Independent fixture review confirms expected outcomes.

Verification evidence: Pending.

## FT-204 — Add alert ranking, lifecycle, and deduplication

Status: Not started

Branch: `feat/ft-204-alert-lifecycle`
Final commit: Pending
Pull request: Pending

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
