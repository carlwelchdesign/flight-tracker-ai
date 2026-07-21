# FT-408 — Public NOAA weather map layers

Status: In progress

Branch: `feat/ft-408-public-weather-map-layers`
Latest implementation commit: Pending
Final commit: Pending
Pull request: Pending
Owner: Full-stack product engineering

## Outcome

Put the existing NOAA aviation-weather capability onto the public navigable
tracker. Recruiters can independently toggle airport/METAR markers and SIGMET
hazard polygons, inspect source-supplied weather evidence, and understand
freshness or failure without signing in or losing the aircraft picture.

The public boundary is fixed to the configured portfolio operator and exposes
only presentation facts. It does not expose tenant identifiers, source-envelope
identifiers, raw provider payloads, protected evidence routes, or arbitrary
operator/region queries.

Dependency: FT-405 live navigable tracker and the completed FT-201/FT-202 NOAA
ingestion/read contracts. This ticket is intentionally stacked on the active
FT-405 feature branch.

## Acceptance checklist

### Public Rust weather boundary

- [x] Rust exposes one no-store public weather snapshot fixed to the configured
      NOAA portfolio operator.
- [x] The response contains bounded latest METAR station facts, bounded latest
      SIGMET hazard revisions, sanitized NOAA source health, generated time,
      and visible NOAA attribution.
- [x] Tenant IDs, source-envelope IDs, raw payloads, protected evidence URLs,
      and arbitrary operator/region parameters are absent.
- [x] Disabled configuration and database/read failures fail closed with an
      explicit unavailable response and never cache.
- [x] Rust tests cover disabled/failure behavior and API/PostGIS smoke proves
      operator binding, bounded data, geometry, source health, and sanitization.

### Web boundary and state

- [x] A public same-origin Next.js route forwards the Rust snapshot without
      credentials and preserves `no-store`.
- [x] Runtime parsing rejects malformed coordinates, geometry, measurements,
      times, lifecycle values, severity, categories, and source health.
- [x] Weather refreshes independently from aircraft positions, retains the last
      accepted layer on failure, and visibly distinguishes loading, current,
      stale, degraded, unavailable, and empty states.
- [x] Parser, proxy, refresh, retained-layer, and unavailable-state tests pass.

### MapLibre presentation

- [x] Independent controls toggle airport/METAR markers and SIGMET polygons
      with visible counts and source/freshness evidence.
- [x] METAR markers encode flight category and expose station, observation age,
      wind, visibility, ceiling, provider, and feed through a selectable panel.
- [x] Hazard polygons encode severity/lifecycle and expose type, validity,
      altitude band, issue time, provider, and feed through a selectable panel.
- [x] Aircraft markers, selected trajectory, weather, airports, controls, and
      attribution remain legible together on desktop and mobile.
- [x] Empty or unavailable weather never removes aircraft, fabricates weather,
      or mislabels retained evidence as current.

### Verification and delivery

- [x] Focused and full Rust/web tests, formatting, Clippy, audit, typecheck,
      lint, production build, API/PostGIS smoke, and whitespace checks pass.
- [x] Browser verification proves live layers, toggles, selection, freshness,
      retained/unavailable behavior, keyboard use, and responsive layout.
- [ ] The dedicated branch has intentional commits, a stacked pull request,
      passing required checks, production deployment evidence, and updated
  ticket/status documentation.

## Verification evidence

- Rust: 92 library tests, 13 binary tests, public weather contract, integration
  suites, formatting, and Clippy with warnings denied pass. The PostGIS contract
  runs against `TEST_DATABASE_URL` in CI and skips only in unit-only shells.
- Web: 91 tests across 27 files, typecheck, lint, production dependency audit,
  and the Next.js production build pass on Node 20.20.1.
- Browser: the production build rendered two live aircraft, two NOAA airport
  observations, and one active SIGMET. Independent toggles, keyboard selection,
  station wind/visibility/ceiling/source evidence, hazard lifecycle/validity/
  altitude/source evidence, attribution, canvas rendering, and absence of an
  error overlay were verified. Retained/unavailable behavior is covered by the
  component integration tests without fabricating network data.

## Non-goals

- Do not add radar imagery, forecast-model tiles, turbulence prediction, route
  optimization, or operational decision authority.
- Do not make protected raw NOAA payload evidence public.
- Do not couple aircraft-source availability to weather-source availability.
