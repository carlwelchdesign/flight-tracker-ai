# FT-411 — Atmospheric weather overlays

Status: In progress

Branch: `feat/ft-411-atmospheric-weather-overlays`
Latest implementation commit: Pending
Final implementation commit: Pending
Pull request: Pending
Owner: Backend and full-stack product engineering

## Outcome

Add legible, source-attributed atmospheric context to the public navigable
flight map: NOAA nowCOAST radar and satellite cloud imagery, NOAA surface-wind
barbs, and an animated upper-air wind field from a bounded weather-model grid.

The overlays are advisory portfolio context, not route-planning or certified
weather products. Controls, timestamps, altitude/pressure level, source,
loading, stale, and unavailable states must remain explicit. Aircraft,
trajectory, METAR, and SIGMET layers must continue to work independently when
an atmospheric source fails.

## Acceptance checklist

### Source boundaries

- [x] NOAA nowCOAST WMS URLs are fixed to HTTPS, named allowlisted layers, and
      transparent Web Mercator tiles; no arbitrary WMS proxy is introduced.
- [x] Rust owns an allowlisted regional/pressure-level model-wind endpoint,
      returns only sanitized vector samples, bounds upstream work, and prevents
      request stampedes with a short in-memory refresh window.
- [x] Radar, satellite, surface wind, and model wind disclose source, product,
      observation/forecast time, level, and non-operational status.
- [x] Upstream failures are isolated and visible without removing aircraft,
      trajectories, METARs, SIGMETs, or the last accepted bounded wind field.

### Public experience

- [x] Keyboard-accessible controls toggle radar, satellite clouds, surface
      winds, and animated model winds independently.
- [x] Upper-air winds offer a small allowlist of clearly labeled pressure/
      approximate-altitude levels and refresh for the selected traffic region.
- [x] The wind animation respects reduced-motion preferences and does not block
      map pan, zoom, rotation, aircraft selection, or weather evidence controls.
- [x] Layer order, opacity, legends, responsive layout, and source copy keep
      aircraft and selected trajectories legible on desktop and mobile.

### Verification and delivery

- [x] Rust tests prove region/level rejection, bounded model parsing, cache
      behavior, sanitization, and unavailable/retained states.
- [x] Web tests prove WMS configuration, toggles, level selection, parser
      rejection, reduced-motion behavior, and independent failure handling.
- [x] Formatting, Clippy, lint, typecheck, unit tests, production build, and
      API/PostGIS smoke pass.
- [ ] Runtime browser verification covers live raster tiles, at least two wind
      levels, region switching, aircraft selection, and one mobile viewport.
- [ ] Ticket branch, intentional commits, PR, passing checks, hosted promotion,
      and verification evidence are recorded before completion.

## Non-goals

- Filed-route weather intersection, turbulence/icing prediction, lightning,
  vertical profiles, historical animation, certified briefing, or provider
  guarantees.
- Persisting raster tiles or model samples, accepting arbitrary coordinates,
  or representing forecast winds as observed aircraft conditions.

## Current verification evidence

- Rust formatting and strict Clippy pass. The complete Rust workspace passes
  100 library tests, 13 binary tests, and all integration and contract tests.
- The complete web suite passes 108 tests across 33 files, including the
  reduced-motion canvas test. Typecheck, zero-warning lint, and the
  production Next.js build pass with the public atmosphere route present.
- NOAA WMS definitions use three fixed nowCOAST products and retain MapLibre's
  literal Web Mercator tile-bounds token. Open-Meteo responses expose the
  provider, model, forecast time, source link, and CC BY 4.0 license link.
- Hosted runtime, PR, CI, merge, and promotion evidence remain pending.
