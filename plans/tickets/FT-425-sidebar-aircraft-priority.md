# FT-425 — Prioritize the aircraft list in the tracker sidebar

Status: Complete

Branch: `fix/ft-425-sidebar-priority`
Final implementation commit: `ba478c3`
Pull request: [#70](https://github.com/carlwelchdesign/flight-tracker-ai/pull/70)
Owner: Frontend engineering and product design

Rebalance the desktop tracker sidebar so the aircraft list is immediately
useful instead of being pushed toward the bottom by an oversized selected-
aircraft panel. Keep the selected aircraft above the list, show its essential
motion facts at a glance, and move secondary evidence into progressive
disclosure.

Dependencies: FT-415, FT-422

## Acceptance checklist

- [x] Selected aircraft remains above the current-picture aircraft list in the
      document and keyboard order.
- [x] The desktop sidebar gives more usable height to the aircraft list than
      the selected-aircraft panel while preserving bounded scrolling.
- [x] Altitude, ground speed, heading, and freshness remain visible for the
      selected aircraft without opening another control.
- [x] Secondary position, timing, provider, trajectory, source-quality, and
      explanatory evidence moves behind an accessible details disclosure.
- [x] Replay attention evidence remains immediately visible when it is
      available; the uninformative live “not evaluated” state moves into the
      secondary disclosure.
- [x] Empty, live, replay, narrow-screen, keyboard, and screen-reader behavior
      remain coherent.
- [x] Focused tests, full web tests, lint, typecheck, production build,
      responsive browser verification, and diff hygiene pass.
- [x] Branch, commits, pull request, required checks, and hosted evidence are
      recorded before completion.

## Non-goals

- Changing aircraft selection, search, sharing, map, replay, or API behavior.
- Removing evidence fields or the route-comparison panel.
- Moving selected aircraft below the aircraft list.

## Verification evidence

- `npm test -- --run src/components/operations/public-flight-tracker-demo.test.tsx`
  — 13 focused tracker tests passed, including hidden/expanded live evidence
  and immediately visible replay attention behavior.
- `npm test` — 47 files and 154 tests passed.
- `npm run lint`, `npm run typecheck`, and `npm run build` passed on Node.js
  24.18.0; `git diff --check` passed.
- Desktop browser verification measured a bounded 580px sidebar with a 220px
  selected-aircraft panel and 352px aircraft-list panel. Expanding secondary
  details increased only the inspector's internal scroll height; the aircraft
  list remained fixed at the same position and height.
- Browser verification at the default desktop viewport and a 390-by-844 narrow
  viewport confirmed correct order, responsive stacking, keyboard-operable
  disclosure, visible secondary evidence after expansion, and no browser
  errors.
- Implementation commit: `ba478c3`; delivery PR:
  [#70](https://github.com/carlwelchdesign/flight-tracker-ai/pull/70).
- GitHub checks passed: API and PostGIS smoke test, Rust checks, Web checks,
  Vercel, and Vercel Preview Comments. PR #70 merged as `bb294ed`.
- Production deployment `dpl_4D5NeyPECy9xhQ8YyEDcjCB6T3Ur` is ready and
  assigned to [flight-tracker-ai-one.vercel.app](https://flight-tracker-ai-one.vercel.app/).
  Hosted verification with 179 aircraft measured a 220px inspector and 352px
  aircraft-list panel before and after expanding secondary details. The
  disclosure exposed provider evidence, the list position remained stable,
  browser errors were empty, and the Vercel error-log scan was clean.
