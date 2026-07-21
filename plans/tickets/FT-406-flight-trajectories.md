# FT-406 — Observed trails and estimated trajectories

Status: Complete

Branch: `feat/ft-406-flight-trajectories`
Latest implementation commit: `8bce8aa`
Final commit: `8bce8aa`
Pull request: [#26](https://github.com/carlwelchdesign/flight-tracker-ai/pull/26)
Owner: Frontend product engineering

## Outcome

Help viewers understand where a selected live aircraft has been and where its
current motion vector points. The navigable map draws a bounded recent trail
from accepted source observations and a separately styled short-term geometric
projection from the latest supplied heading and ground speed.

The observed trail is not a persisted flight history. It exists only in page
memory and is discarded on reload. The projected line is not a filed route,
destination prediction, safety recommendation, or new ADS-B observation.

Dependency: FT-405 live navigable tracker. This ticket is intentionally stacked
on its feature branch until FT-405 merges.

## Acceptance checklist

### Deterministic trajectory policy

- [x] A pure TypeScript policy accepts ordered live observations and maintains
      at most ten minutes and 25 points per aircraft.
- [x] Duplicate and out-of-order observations cannot rewind or inflate a trail.
- [x] Missing aircraft histories are pruned within the same bounded window.
- [x] A deterministic geodesic projection uses only the latest supplied
      position, true heading, and ground speed, with a documented five-minute
      horizon and safe handling for missing or invalid motion facts.
- [x] Unit tests cover accumulation, deduplication, ordering, pruning, bounds,
      projection direction/distance, and missing motion facts.

### Map presentation and truthfulness

- [x] Selecting a live aircraft draws its recent observed trail as a solid line
      and its estimated five-minute projection as a visually distinct dashed
      line with an endpoint marker.
- [x] Map/list selection updates trajectory layers without recreating the map.
- [x] A visible legend and aircraft evidence panel distinguish `Observed trail`
      from `Estimated 5-min projection`.
- [x] Replay, unavailable motion facts, and a single accepted observation do
      not fabricate a live trail or projected route.
- [x] Trajectory layers remain legible in the dark basemap, keyboard selection
      remains usable, and reduced-motion behavior remains intact.

### Data and release boundary

- [x] ADSB.lol positions used for trails remain page-memory-only and are not
      written to browser storage, PostgreSQL, logs, analytics, exports, URLs, or
      an LLM.
- [x] No trajectory line implies origin, destination, airline identity, filed
      route, arrival time, conflict prediction, or operational authority.
- [x] Focused tests, full web tests, typecheck, lint, production build, and
      whitespace checks pass.
- [x] Desktop and mobile browser verification proves live accumulation,
      selection changes, layer styling, responsive layout, and no application
      errors.
- [x] The dedicated branch has intentional commits, a stacked pull request,
      passing required checks, and updated ticket/status evidence.

## Implementation notes

- Keep history policy and geodesic math in a framework-independent module.
- Keep snapshot polling ownership in the public tracker component and MapLibre
  source/layer ownership in the map component.
- Prefer one selected-aircraft trail and projection over rendering hundreds of
  overlapping tracks.
- Keep source timestamps unchanged. Presentation animation and estimated
  projection must never mutate or replace source evidence.

## Verification evidence

- Pure policy tests cover ordered accumulation, duplicate/out-of-order rejection,
  ten-minute pruning, 25-point bounding, missing-aircraft pruning, north/east
  geodesic projection, exact distance/horizon evidence, and missing/invalid
  motion facts.
- `npm ci`, `npm audit --omit=dev --audit-level=moderate`, 68 web tests across
  22 files, TypeScript, ESLint, `next build`, and `git diff --check` pass on
  Node `20.20.1`. GitHub advisory
  [GHSA-f88m-g3jw-g9cj](https://github.com/advisories/GHSA-f88m-g3jw-g9cj)
  appeared during delivery; commit `8bce8aa` overrides Next's transitive Sharp
  runtime to patched `0.35.3`, after which the audit reports zero vulnerabilities.
- A production Next build running against the live staging Rust API began with
  `Starts after next refresh` and a supplied 1.5 NM estimate. After the next
  source cadence, selecting `PCM7700` displayed two source points, a solid
  fading observed trail, a dashed 11.1 NM five-minute projection, an endpoint,
  and matching inspector evidence without recreating the map.
- Browser verification proves live/map/list selection, truthfulness copy,
  OpenFreeMap rendering, no framework overlay, and no application errors. At
  `390x844`, the map is 460 pixels tall, the aircraft panel is bounded to 560
  pixels with internal scrolling, and horizontal overflow is zero.
- CI run [29874867026](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29874867026)
  passes Rust, web, API/PostGIS, Vercel, and Vercel Preview Comments for commit
  `8bce8aa`. The ready preview deployment is
  `https://flight-tracker-qyece7zgf-carlwelchdesigns-projects.vercel.app` and
  remains protected by the project's preview-access policy.
