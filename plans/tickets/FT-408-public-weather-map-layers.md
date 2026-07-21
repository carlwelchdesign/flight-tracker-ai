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

- [ ] Rust exposes one no-store public weather snapshot fixed to the configured
      NOAA portfolio operator.
- [ ] The response contains bounded latest METAR station facts, bounded latest
      SIGMET hazard revisions, sanitized NOAA source health, generated time,
      and visible NOAA attribution.
- [ ] Tenant IDs, source-envelope IDs, raw payloads, protected evidence URLs,
      and arbitrary operator/region parameters are absent.
- [ ] Disabled configuration and database/read failures fail closed with an
      explicit unavailable response and never cache.
- [ ] Rust tests cover disabled/failure behavior and API/PostGIS smoke proves
      operator binding, bounded data, geometry, source health, and sanitization.

### Web boundary and state

- [ ] A public same-origin Next.js route forwards the Rust snapshot without
      credentials and preserves `no-store`.
- [ ] Runtime parsing rejects malformed coordinates, geometry, measurements,
      times, lifecycle values, severity, categories, and source health.
- [ ] Weather refreshes independently from aircraft positions, retains the last
      accepted layer on failure, and visibly distinguishes loading, current,
      stale, degraded, unavailable, and empty states.
- [ ] Parser, proxy, refresh, retained-layer, and unavailable-state tests pass.

### MapLibre presentation

- [ ] Independent controls toggle airport/METAR markers and SIGMET polygons
      with visible counts and source/freshness evidence.
- [ ] METAR markers encode flight category and expose station, observation age,
      wind, visibility, ceiling, provider, and feed through a selectable panel.
- [ ] Hazard polygons encode severity/lifecycle and expose type, validity,
      altitude band, issue time, provider, and feed through a selectable panel.
- [ ] Aircraft markers, selected trajectory, weather, airports, controls, and
      attribution remain legible together on desktop and mobile.
- [ ] Empty or unavailable weather never removes aircraft, fabricates weather,
      or mislabels retained evidence as current.

### Verification and delivery

- [ ] Focused and full Rust/web tests, formatting, Clippy, audit, typecheck,
      lint, production build, API/PostGIS smoke, and whitespace checks pass.
- [ ] Browser verification proves live layers, toggles, selection, freshness,
      retained/unavailable behavior, keyboard use, and responsive layout.
- [ ] The dedicated branch has intentional commits, a stacked pull request,
      passing required checks, production deployment evidence, and updated
      ticket/status documentation.

## Non-goals

- Do not add radar imagery, forecast-model tiles, turbulence prediction, route
  optimization, or operational decision authority.
- Do not make protected raw NOAA payload evidence public.
- Do not couple aircraft-source availability to weather-source availability.

