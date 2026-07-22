# FT-405 — Live navigable flight tracker

Status: In progress — live core deployed; overlay and forced-failure verification remain

Branch: `feat/ft-405-live-navigable-tracker`
Closeout branch: `test/ft-405-live-map-verification`
Latest implementation commit: `27bcec3`
Final commit: Pending
Pull request: [#25](https://github.com/carlwelchdesign/flight-tracker-ai/pull/25)
Owner: Full-stack product engineering

## Outcome

Replace the fixed public SVG/replay presentation with the product users expect
from a flight tracker: a pannable, zoomable, rotatable geographic map showing a
bounded region of best-effort aircraft positions from ADSB.lol. Aircraft move
smoothly between provider snapshots, can be selected for live position and
motion details, and visibly fall back to the deterministic replay when the free
source is unavailable.

"Live" means the latest best-effort ADSB.lol regional snapshot, polled no more
frequently than once every 30 seconds under the existing provider policy. The UI
may interpolate marker motion between accepted snapshots, but must never imply
that interpolation is a new source observation. ADSB.lol does not provide
authoritative routes, schedules, delays, or complete coverage.

## Acceptance checklist

### Public data boundary

- [x] Rust exposes a public, operator-bound, sanitized live-position snapshot
      containing only aircraft identity, position, motion, observation time,
      freshness, source state, region, and required attribution. Evidence:
      `tests::public_live_positions_are_operator_bound_and_sanitized`.
- [x] The endpoint is fixed to the configured portfolio operator and cannot be
      used to enumerate tenants, choose arbitrary regions, or request individual
      aircraft. Evidence: fixed runtime injection plus Rust HTTP contract test.
- [x] Raw and normalized ADSB.lol records remain ephemeral and are never written
      to PostgreSQL, logs, analytics, browser storage, exports, or backups.
- [x] Rust and Next.js preserve `Cache-Control: no-store`, bounded response size,
      provider attribution, and fail-closed configuration.
- [x] Production enables one bounded ADSB.lol region with a 30-second-or-slower
      poll interval and a project-identifying user agent.

### Navigable map

- [x] MapLibre GL JS replaces the fixed SVG as the primary public map.
- [x] Users can pan, zoom, rotate, use keyboard controls, reset north, and fit
      the camera to currently visible traffic.
- [x] The basemap uses a keyless provider approved for this portfolio demo and
      shows required OpenStreetMap/OpenFreeMap attribution.
- [ ] Aircraft, weather hazards, airports, and selected state remain legible
      against the dark basemap across desktop and mobile layouts.
- [x] The flight list and map selection stay synchronized.

### Realtime motion and truthfulness

- [x] The browser refreshes the sanitized snapshot at the provider cadence and
      updates aircraft without a page reload.
- [x] Aircraft position and heading animate smoothly between accepted snapshots;
      reduced-motion users receive immediate non-animated updates.
- [x] The UI distinguishes observed time, received time, snapshot age, stale
      aircraft, provider state, and interpolated presentation.
- [x] Callsign, altitude, speed, heading, vertical rate, squawk, and source
      quality are shown only when supplied by the live position source.
- [x] No live aircraft is assigned a fabricated origin, destination, route,
      schedule, delay, airline, or operational status.

### Failure and fallback

- [x] Connecting, current, stale, degraded, unavailable, and disabled states are
      visible without clearing the last accepted picture.
- [x] If live data is unavailable, the public map offers and automatically
      preserves a clearly labeled deterministic replay demonstration.
- [x] Retrying live data does not restart the page or discard the selected
      aircraft unnecessarily.

### Verification and release

- [x] Rust contract, tenant-boundary, no-store, payload-limit, stale-data, and
      provider-failure tests pass.
- [ ] Web parser, polling, interpolation, reduced-motion, selection, fallback,
      accessibility, and MapLibre lifecycle tests pass.
- [x] Lint, typecheck, unit tests, production build, Rust formatting, Clippy,
      and API/PostGIS smoke pass.
- [ ] Browser verification proves pan, zoom, aircraft selection, live refresh,
      animation, attribution, degraded fallback, and responsive behavior on the
      deployed candidate.
- [x] The feature branch has intentional commits, a pull request, passing CI,
      production deployment evidence, and an updated checklist before merge.

## Implementation notes

- Reuse the existing Rust ADSB.lol normalizer, status store, fleet projection,
  one-request-in-flight policy, retry/backoff, and ODbL attribution contract.
- Prefer a small public read model over exposing authenticated operational APIs.
- Use a MapLibre GeoJSON source/layer for aircraft so selection and bulk updates
  remain efficient. Interpolation belongs in the presentation layer and must
  not alter stored or source timestamps.
- Keep deterministic replay as a product-quality fallback, not as the default
  impression when current live data is available.

## Current evidence

- Implementation commits `3da313b`, `58da52d`, and `27bcec3` add the sanitized Rust endpoint, same-origin
  public proxy, MapLibre/OpenFreeMap experience, 30-second polling, bounded
  interpolation, source states, selection, truthful details, received/observed
  evidence, and replay fallback.
- Local verification: Rust formatting, 90 Rust tests, Clippy with warnings denied,
  TypeScript, ESLint, 64 web tests, and `next build` pass. One parallel-suite
  alert test timed out once, then passed in isolation and in the complete
  single-worker suite; it is unrelated to the live tracker.
- Browser verification at desktop and 390x844 proves the basemap, attribution,
  map controls, aircraft markers, list/detail selection, zoom interaction, replay
  fallback, no framework overlay, a 460-pixel mobile map, and zero mobile
  horizontal overflow.
- CI run [29873257420](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29873257420)
  passes Rust, web, API/PostGIS, and Vercel checks for commit `27bcec3`.
- Render production and staging run the bounded 50-nautical-mile Bay Area feed
  with a 30-second poll. The hosted source advanced from `22:22:44.001Z` with
  143 aircraft to `22:23:14.002Z` with 142 aircraft without a page deployment.
- Vercel production deployment `dpl_4YkbL38zNn9qxwZY4iwEkvC1LB1t` is assigned
  to `https://flight-tracker-ai-one.vercel.app`. A production probe returned
  `current`, 153 tracked aircraft, 139 fresh positions, and newest observation
  `22:19:44Z`. Browser verification proved the navigable map, OpenFreeMap and
  OpenStreetMap attribution, live list/detail selection, zoom, evidence fields,
  interpolation disclosure, production Clerk keys, and no application errors.
  Vercel's separate Clerk custom-domain DNS integration check remains red even
  though the configured Vercel-domain Clerk flow and production app are working.

## Remaining before ticket completion

- Public airport/METAR and SIGMET overlays are owned by stacked follow-up
  [`FT-408-public-weather-map-layers.md`](FT-408-public-weather-map-layers.md).
- Observed trails and estimated short-term trajectories are owned by stacked
  follow-up [`FT-406-flight-trajectories.md`](FT-406-flight-trajectories.md).
- Add airport and NOAA hazard/weather overlays to the navigable map and verify
  their selected/disabled states on desktop and mobile.
- Add direct unit coverage for marker interpolation, reduced-motion behavior,
  and MapLibre setup/cleanup; current coverage combines component contracts with
  real-browser verification.
- Exercise forced degraded and disabled responses against a deployed candidate;
  local component/browser fallback is verified, while production was kept on
  current live data during the release smoke.
