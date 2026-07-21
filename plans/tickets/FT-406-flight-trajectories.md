# FT-406 — Observed trails and estimated trajectories

Status: In progress

Branch: `feat/ft-406-flight-trajectories`
Latest implementation commit: Pending
Final commit: Pending
Pull request: Pending
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

- [ ] A pure TypeScript policy accepts ordered live observations and maintains
      at most ten minutes and 25 points per aircraft.
- [ ] Duplicate and out-of-order observations cannot rewind or inflate a trail.
- [ ] Missing aircraft histories are pruned within the same bounded window.
- [ ] A deterministic geodesic projection uses only the latest supplied
      position, true heading, and ground speed, with a documented five-minute
      horizon and safe handling for missing or invalid motion facts.
- [ ] Unit tests cover accumulation, deduplication, ordering, pruning, bounds,
      projection direction/distance, and missing motion facts.

### Map presentation and truthfulness

- [ ] Selecting a live aircraft draws its recent observed trail as a solid line
      and its estimated five-minute projection as a visually distinct dashed
      line with an endpoint marker.
- [ ] Map/list selection updates trajectory layers without recreating the map.
- [ ] A visible legend and aircraft evidence panel distinguish `Observed trail`
      from `Estimated 5-min projection`.
- [ ] Replay, unavailable motion facts, and a single accepted observation do
      not fabricate a live trail or projected route.
- [ ] Trajectory layers remain legible in the dark basemap, keyboard selection
      remains usable, and reduced-motion behavior remains intact.

### Data and release boundary

- [ ] ADSB.lol positions used for trails remain page-memory-only and are not
      written to browser storage, PostgreSQL, logs, analytics, exports, URLs, or
      an LLM.
- [ ] No trajectory line implies origin, destination, airline identity, filed
      route, arrival time, conflict prediction, or operational authority.
- [ ] Focused tests, full web tests, typecheck, lint, production build, and
      whitespace checks pass.
- [ ] Desktop and mobile browser verification proves live accumulation,
      selection changes, layer styling, responsive layout, and no application
      errors.
- [ ] The dedicated branch has intentional commits, a stacked pull request,
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

Pending.
