# Project Status

Last updated: 2026-07-22

## Current state

- Current milestone: M5 — Optimization research and controlled recommendations
- Active tickets: FT-403 neutral recruiter validation and FT-502 independent
  aviation-domain review
- Branch: external evidence branches as those sessions become available
- Pull request: FT-504 [#64](https://github.com/carlwelchdesign/flight-tracker-ai/pull/64)
  is merged
- Owner: Project owner and external reviewers
- Overall status: M0, M1, M2, and M3 are complete; M4 is 3/4 complete,
  M4.1 engineering is 5/5 complete, and M5 is 2/3 complete. Neutral recruiter
  validation and FT-502 independent aviation-domain review remain external
  gates.
- Next action: Run the remaining neutral recruiter session and independent
  aviation-domain review without weakening either external evidence gate.
- Sequencing exception: On 2026-07-22 the project owner explicitly authorized
  FT-503 engineering to proceed while FT-502's independent domain review
  remains pending; the review requirement itself is unchanged.

## Milestone checklist

- [x] M0 — Foundation, feasibility, and portfolio rebaseline
- [x] M1 — Simulated operations console
- [x] M2 — Live weather and hazard intelligence
- [x] M3 — Portfolio live data and operational workflow
- [ ] M4 — Portfolio launch and demonstration hardening
- [ ] M4.1 — Public decision intelligence and exploration
- [ ] M5 — Optimization research and controlled recommendations

## Current product correction

- PR [#25](https://github.com/carlwelchdesign/flight-tracker-ai/pull/25) is
  merged through PR [#24](https://github.com/carlwelchdesign/flight-tracker-ai/pull/24)
  into `main`. FT-410 now owns the correction from one fixed SFO feed to a
  curated set of bounded live-traffic regions. See
  [`FT-410-regional-traffic-selector.md`](tickets/FT-410-regional-traffic-selector.md).

- FT-408 is complete and merged through stacked PR [#28](https://github.com/carlwelchdesign/flight-tracker-ai/pull/28).
  It ports NOAA METAR and SIGMET capability to the public navigable map through
  a fixed-operator sanitized Rust read boundary and is live in production. See
  [`FT-408-public-weather-map-layers.md`](tickets/FT-408-public-weather-map-layers.md).
- FT-407 is complete and merged through stacked PR [#27](https://github.com/carlwelchdesign/flight-tracker-ai/pull/27). It corrects
  the public live-map glyph axis without changing ADS-B headings, trajectory
  math, or the protected console's separate north-facing SVG marker. See
  [`FT-407-aircraft-marker-heading.md`](tickets/FT-407-aircraft-marker-heading.md).
- FT-406 is complete and merged through stacked PR [#26](https://github.com/carlwelchdesign/flight-tracker-ai/pull/26). It adds a selected live
  aircraft's ten-minute observed trail and a separately labeled estimated
  five-minute motion projection without persisting provider positions or
  claiming a filed route, destination, ETA, or authoritative prediction. See
  [`FT-406-flight-trajectories.md`](tickets/FT-406-flight-trajectories.md).
- FT-405 is complete through implementation PR #25 and closeout PR #54. The
  original public SVG/replay surface was replaced by a sanitized public
  no-store ADSB.lol read model, a navigable MapLibre/OpenFreeMap view, animated
  aircraft updates, truthful evidence fields, and an explicit replay fallback.
  Follow-up tickets added trajectories, weather layers, regional traffic,
  replay telemetry, and airport intelligence. Direct lifecycle/motion tests and
  an isolated hosted source-failure check close the former gaps. See
  [`FT-405-live-navigable-tracker.md`](tickets/FT-405-live-navigable-tracker.md).

## Ticket progress

| Milestone | Complete | Total |
| --- | ---: | ---: |
| M0 | 5 | 5 |
| M1 | 4 | 4 |
| M2 | 4 | 4 |
| M3 | 4 | 4 |
| M4 | 3 | 4 |
| M4.1 | 5 | 5 |
| M5 | 2 | 3 |

## Handoff notes

- FT-504 is complete through implementation commit `063f2df`, PR
  [#64](https://github.com/carlwelchdesign/flight-tracker-ai/pull/64), and merge
  commit `b88b9e3`. The production portfolio now exposes the existing FT-503
  Responses API work through one fixed synthetic case, with no arbitrary
  prompt, live/provider data, approval, send, or operational action path. Rust
  retains evidence minimization, validation, deterministic fallback, and the
  mandatory `awaiting_review` state. GitHub Actions run
  [29945949109](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29945949109)
  passed all required checks. Render production deploy
  `dep-d9ggn5btqb8s73cula80` returned an actual `gpt-5.6-luna` draft with no
  fallback and stable cached response bodies. Vercel production deployment
  `dpl_HrCfTqKN1bmDReYh76KUzgLaESre` is assigned to the public domain; its live
  browser flow shows the model and human-review state with no approve or send
  controls.

- FT-422 is complete through implementation commit `3485e3f` and PR
  [#63](https://github.com/carlwelchdesign/flight-tracker-ai/pull/63). The NOAA
  layers and airport-intelligence surfaces are independently closable, bounded
  draggable desktop panels with focus stacking and one accessible recovery
  menu. Narrow viewports stack both panels in document flow and disable drag.
  All 147 web tests, lint, TypeScript, production build, diff hygiene, desktop
  and 390-pixel browser checks pass. GitHub Actions run
  [29944316820](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29944316820)
  and the Vercel preview passed.

- FT-421 is complete through implementation commit `2bd1349` and PR
  [#62](https://github.com/carlwelchdesign/flight-tracker-ai/pull/62). FT303's
  replay positions now follow its supplied northwest heading, eliminating the
  contradictory northeast trail that made the glyph appear rotated. A new
  great-circle-bearing regression protects both segments while the live marker
  transform and operational policy remain unchanged. Strict Clippy, Rust
  formatting, the complete Rust and web test suites, ESLint, TypeScript,
  production build, diff hygiene, and corrected-timeline browser verification
  pass. GitHub Actions run
  [29942050285](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29942050285)
  passed all required checks, and the Vercel preview completed successfully.

- FT-420 is complete through implementation commit `5cba771` and PR
  [#61](https://github.com/carlwelchdesign/flight-tracker-ai/pull/61). The public
  footer now provides accessible, dependency-free LinkedIn and GitHub SVG links
  with keyboard focus, hover feedback, safe new-tab behavior, and responsive
  layout. All 143 web tests, lint, TypeScript, production build, diff hygiene,
  and desktop/mobile browser verification pass. GitHub Actions run
  [29941214012](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29941214012)
  passed all required checks, and the Vercel preview completed successfully.

- FT-419 is complete through documentation commit `0e4c6f1` and PR
  [#60](https://github.com/carlwelchdesign/flight-tracker-ai/pull/60). The README
  now leads with the production product, accurately describes the shipped
  exploration and decision-intelligence features, distinguishes live, replay,
  deterministic, and human-reviewed AI boundaries, and retains concise setup
  and architecture guidance. Current production replay screenshots cover a
  1440 by 1000 desktop viewport and a 390 by 844 mobile viewport with no browser
  errors. GitHub Actions run
  [29940600554](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29940600554)
  passed all required checks, and the Vercel preview completed successfully.

- FT-418 is complete through implementation commit `6a9adc9`, PR
  [#58](https://github.com/carlwelchdesign/flight-tracker-ai/pull/58), and merge
  commit `af3dbb6`. The web
  package, local pin, CI, Docker image, Node types, lockfile, and current
  runbooks now use Node.js 24. Vercel project settings also select `24.x`.
  CI run
  [29939055952](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29939055952)
  passed all required checks. Cache-free preview deployment
  `dpl_H1YnJnYCsfijHyEyqGS1KwFoksBS` completed `Ready` without the Node 20
  deprecation, Mapbox engine mismatch, or stale Vercel setting warning, and its
  authenticated tracker/image probes returned HTTP 200.
  Main CI run
  [29939552062](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29939552062)
  also passed after merge. Production deployment
  `dpl_H4D8NAar318egoMPZmb3brVZQ7Dk` from merged `main` is assigned to
  `flight-tracker-ai-one.vercel.app`; its clean Node 24 log contains none of
  the reported errors, and anonymous tracker/image probes return HTTP 200.

- FT-417 is complete through implementation commit `8715401`, PR
  [#56](https://github.com/carlwelchdesign/flight-tracker-ai/pull/56), and merge
  commit `515b32e`. The root
  page now publishes canonical, Open Graph, and Twitter/X metadata plus a
  generated 1200 by 630 branded PNG. CI run
  [29937128296](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29937128296)
  passed all required checks. Production deployment
  `dpl_5WsRREDHHuNMiT8mwt8MYP8FA4q7` from merged `main` is assigned to
  `flight-tracker-ai-one.vercel.app`; anonymous root and image requests both
  return HTTP 200, and the image route does not redirect through Clerk.
  Main-branch CI run
  [29937558009](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29937558009)
  passed Rust, web, and API/PostGIS checks after merge.

- FT-405 closeout commit `b6a5944` adds direct polling, MapLibre lifecycle,
  marker selection, interpolation, reduced-motion, cleanup, and map-failure
  coverage. All 140 web tests, lint, TypeScript, and the clean production build
  pass. Hosted Preview deployment `GENwMrgCEMzCfH4VVXZR3bEG7jkV` switched to
  the deterministic fallback under an isolated branch-only API failure,
  retained three aircraft after retry, fit the 390 by 844 mobile viewport, and
  logged zero browser errors. The temporary Vercel override was removed.
  Final CI run
  [29936224809](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29936224809)
  passed all required checks before PR #54 merged as `03487e1`.

- FT-503 is complete and merged through PR
  [#52](https://github.com/carlwelchdesign/flight-tracker-ai/pull/52) at merge
  commit `9811eb0`. It proceeded with explicit owner
  authorization to proceed while FT-502's external domain review remains
  pending. Rust will keep source-fact minimization, draft validation,
  deterministic fallback, and review state separate from the optional OpenAI
  Responses API adapter. No model output can select a route, alter eligibility,
  approve itself, or trigger a message or operational action.
  Implementation and local verification are complete: twelve focused drafting
  tests and all 130 Rust library tests pass, along with binary/integration/
  example tests, strict Clippy, formatting, and diff hygiene. Seven versioned
  evaluation cases matched their expected findings. A live OpenAI probe was
  rate-limited and safely degraded to a deterministic draft that still required
  explicit review; the local credential remains ignored and uncommitted.
  Implementation commit `f7cb5c6` passed GitHub Actions run
  [29934635940](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29934635940)
  passed Rust, web, and API/PostGIS checks; Vercel preview also passed.

- Final production engineering audit on commit `9811eb0` passed the public
  tracker, security headers, API health/readiness, protected-route denial,
  attention explanation, replay timeline, KSFO TAF/PIREP intelligence, SFO live
  positions, NOAA weather, and surface wind. Evidence is captured in
  [`evidence/FINAL_PRODUCTION_AUDIT_2026-07-22.json`](evidence/FINAL_PRODUCTION_AUDIT_2026-07-22.json).
  This does not replace the FT-403 neutral-human session or FT-502 independent
  aviation-domain judgment.

- FT-404 hosted smoke resumed on
  `fix/ft-404-hosted-smoke-closeout`. Production `/health` is healthy, but the
  first current-state check found `/readiness` failing because long-interval
  NOAA and regional ADSB.lol workers did not heartbeat while waiting or
  fetching. The branch adds a shared periodic heartbeat wrapper, updates the
  stale public verifier contract after FT-412, and passes all 101 Rust library
  tests, 13 binary tests, integration/doc tests, strict Clippy, formatting,
  eight verifier tests, and diff hygiene. Implementation commit `7f529e3` is
  published in draft PR
  [#36](https://github.com/carlwelchdesign/flight-tracker-ai/pull/36). Staged
  deployment, production promotion, and authenticated browser/FT-401 evidence
  remain to be recorded.

- The post-launch product direction is now durable in
  [`tickets/M4.1-public-decision-intelligence.md`](tickets/M4.1-public-decision-intelligence.md).
  FT-413 through FT-416 each require an isolated feature branch, intentional
  commits, one PR to `main`, required checks, hosted browser evidence, and
  updated ticket checklists. They remain queued behind completion of FT-403 and
  FT-404 and precede the unchanged FT-501 through FT-503 recommendation work.

- FT-412 is complete and merged into `main` through PR
  [#33](https://github.com/carlwelchdesign/flight-tracker-ai/pull/33) at merge
  commit `914ca72`; implementation commit `0f38fc6` owns the product change. CI run
  [29890580981](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29890580981)
  passes all five checks. Preview `dpl_Ew9JXZngfxCAi4gkaoeenaDWzVBr`
  begins directly with the Flight Tracker AI header, contains neither removed
  section, switches regions successfully, and reports zero preview-origin
  application errors.

- FT-411 is complete and merged into `main` through PR
  [#31](https://github.com/carlwelchdesign/flight-tracker-ai/pull/31) at merge
  commit `9ce391f`; implementation commit `523da95` owns the product change. CI run
  [29882654316](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29882654316)
  passes all five checks. Render staging and production serve the bounded
  Open-Meteo GFS/HRRR wind field, and promoted Vercel deployment
  `dpl_GVX9umVXar24e9gxw4MGd7tvw1H1` is live on the public alias. Production
  browser verification proves radar, satellite, surface-wind, two upper-air
  levels, SFO/LAX switching, aircraft selection, visible timestamps and
  attribution, retained panel order, and zero production-origin application
  errors.

- PR [#30](https://github.com/carlwelchdesign/flight-tracker-ai/pull/30) is
  merged into `main` at `b511643`. The Render blueprint and both services now
  track `main`; the production Rust endpoint returns distinct snapshots for all
  seven regions. Vercel build `EsJMxpjiB` was promoted after the repository CI
  passed, and the public browser switched from San Francisco to Los Angeles
  without reload. FT-411 is active on its dedicated branch.

- FT-410 implementation is locally verified on
  `feat/ft-410-regional-traffic-selector`. Rust owns seven 50-NM airport
  regions with isolated in-memory projections and evenly staggered 75-second
  polling. A production-built browser run switched from 132 current SFO
  aircraft to 173 current LAX aircraft without reload, rendered 173 markers,
  preserved the selected-aircraft-first layout, and had zero mobile horizontal
  overflow. Implementation commit `330d0d4` is published in draft PR
  [#30](https://github.com/carlwelchdesign/flight-tracker-ai/pull/30); required
  CI run [29880472585](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29880472585)
  passes Rust, web, API/PostGIS, and Vercel checks. Promotion commit `09b22d0`
  moves the Render service declarations to `main` while retaining manual
  production deploys.

- FT-409 is complete and merged into FT-405 through PR [#29](https://github.com/carlwelchdesign/flight-tracker-ai/pull/29)
  at `a99c47f`. The selected-aircraft evidence panel now precedes the current
  aircraft list in visual and document order, while the list retains its own
  scrolling region. All five checks pass. Vercel production deployment
  `dpl_GL7BncuJF6ptg8aeBmDXpSvLxLK1` is live, and the public browser check
  confirms the intended order with no application errors or overflow.
- FT-408 is merged into the FT-405 feature branch through PR [#28](https://github.com/carlwelchdesign/flight-tracker-ai/pull/28)
  at merge commit `d222094`; hosted-enablement commit `1ed490c` passes all five
  checks. Render staging and production return a sanitized no-store NOAA
  snapshot with three Bay Area METAR observations and 19 current provider
  hazards. Vercel production deployment
  `dpl_Cv3Z6XafLmTTh7and9SzBcx3fpzi` is live at
  `https://flight-tracker-ai-one.vercel.app`; production browser verification
  proves visible stale/current source truth, independent METAR/SIGMET controls,
  selection evidence, attribution, live aircraft continuity, and no application
  errors. The separate Clerk Marketplace DNS advisory remains unchanged.
- FT-407 is merged into the FT-405 feature branch through PR [#27](https://github.com/carlwelchdesign/flight-tracker-ai/pull/27)
  at merge commit `965a779`. Final CI run
  [29875782640](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29875782640)
  passes all five checks. Production deployment
  `dpl_9CXK2vvtPbdcNiaBnX9ZK52FgPfN` is live at
  `https://flight-tracker-ai-one.vercel.app`. The public live MapLibre marker now applies an
  explicit negative 90-degree glyph-axis correction; browser verification
  proved supplied headings render with the exact correction in local and public
  production without changing source evidence or trajectory math.
- FT-406 is merged into the FT-405 feature branch through PR [#26](https://github.com/carlwelchdesign/flight-tracker-ai/pull/26)
  at merge commit `2df6deb`. Final CI run
  [29875004160](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29875004160)
  passes all five checks. Production deployment
  `dpl_6NHNmJBCJPiaFHrtURRJ8zv8832E` is live at
  `https://flight-tracker-ai-one.vercel.app`. Selected live aircraft now show a bounded ten-minute
  page-memory trail and a separately styled deterministic five-minute motion
  estimate; neither is persisted or represented as a route, destination, ETA,
  or new observation. Live desktop/mobile browser verification captured two
  accepted source points and a 9.6 NM projection after one provider refresh on
  the public production site. Mobile verification confirms a bounded 560-pixel
  aircraft panel and zero horizontal overflow at `390x844`.
- GitHub repository: `carlwelchdesign/flight-tracker-ai`.
- `main` and `origin` are established; planning baseline commit: `8feb57d`.
- FT-001 is merged through PR #1 at `c8e0bb4`.
- The foundation includes the Rust health/readiness boundary, Next.js interface, PostgreSQL/PostGIS migration, production container targets, one-command startup, and green CI.
- FT-002 implementation is merged through PR #3; correction PR #4 is also merged and preserves provider revisions while deduplicating identical deliveries.
- The canonical v1 contract separates raw envelopes from normalized facts, uses explicit UTC time/unit/geometry semantics, and enforces operator consistency through composite foreign keys.
- FT-003 is merged through PR #2 at `7edfa2a`; M0 is complete.
- FT-101 is merged through PR #5 at `efc2cf6` with all required checks passing.
- FT-102 is merged through PR #6 at `aed432d` with all required checks passing.
- FT-103 is merged through PR #7 at `18a5a23` with all required checks passing.
- FT-104 is merged through PR #8 at `da1a6ad` with all required checks passing.
- FT-201 is merged through PR #9 at `2ce50e2`; Rust, web, and API/PostGIS checks pass, including NOAA persistence and revision behavior.
- FT-202 implementation commit `568bd63` and CI correction `225937f` are green in PR #10; Rust, web, and API/PostGIS checks pass, including weather reads and raw-source evidence against PostGIS.
- FT-202 is merged through PR #10 at `617b337`; local `main` was synchronized before FT-203 began.
- FT-203 is active on `feat/ft-203-route-hazard-rules`; its pure Rust domain rule will remain independent of Axum, SQLx, provider payloads, and wall-clock time.
- FT-203 implementation commit `28f227f` is green in PR #11; Rust, web, and API/PostGIS checks pass, and the PostGIS 3.5 oracle independently confirmed all eight golden cases in CI run `29809973027`.
- FT-203 is merged through PR #11 at `848af8f`; local `main` was synchronized before FT-204 began.
- FT-204 is active on `feat/ft-204-alert-lifecycle`; deterministic Rust policy will own ranking, dedupe, transitions, and audit evidence while the web app exposes human-controlled actions.
- FT-204 is delivered through PR #12. CI run `29811831163` verifies live replay persistence, route-hazard alert creation, score evidence, API acknowledgement, schema invariants, and the independent PostGIS rule oracle; M2 is complete.
- FT-301 is re-scoped to choose an officially eligible free, best-effort aircraft-position source or record replay-only as the outcome. Commercial rights, price, SLA, operator contacts, and a 14-day trial no longer block the portfolio roadmap.
- FT-301 is delivered through PR [#20](https://github.com/carlwelchdesign/flight-tracker-ai/pull/20) at selection commit `13d64eb`. CI run [29852787739](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29852787739) passes Rust, web, and API/PostGIS checks. ADR-011 selects ADSB.lol only for an optional ODbL-attributed, ephemeral, regional position layer with `no-store`, no persistence/export/LLM use, and deterministic replay as the only fallback; FT-302 owns implementation and activation proof.
- FT-302 is delivered through PR [#21](https://github.com/carlwelchdesign/flight-tracker-ai/pull/21) at implementation commit `fe8957b`. CI run [29855008220](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29855008220) passes Rust, web, and API/PostGIS checks. The ADSB.lol layer is disabled by default, bounded to one regional request, ephemeral and uncached, visibly ODbL-attributed, honest about position-only facts and coverage, and independent of the deterministic replay fallback. M3 is complete.
- FT-303 is delivered through PR #13 at implementation commit `1430ce8`. CI run `29814499315` verifies Rust, web, live authenticated replay, the identity migration, PostGIS cross-tenant route isolation, session revocation, and actor/tenant audit behavior. Hosted identity remains behind a provider-neutral boundary; tenant membership and operational authorization are app-owned and enforced by Rust.
- FT-304 is delivered through PR #14 at implementation and CI contract commit `11bdc0d`. CI run `29816346733` verifies the additive migration, authenticated replay, workflow-version acknowledgement, tenant-safe assignment, all queue filters, structured dismissal, conflict rejection, bounded persistence volume, Rust and web quality gates, and production builds.
- FT-301 commercial preparation package is merged through PR #15 at `c8d8a78`; PR #16 records the corrected procurement handoff. This research is retained for an optional future production track and is not an active portfolio-release gate.
- FT-004 upgrades checkout and setup-node to their official Node 24 action-runtime releases through PR #17 at implementation commit `715d7d6`; CI run `29832129375` passed all three jobs with zero check annotations while preserving application Node.js `20.20.1`.
- FT-401 is delivered through PR [#18](https://github.com/carlwelchdesign/flight-tracker-ai/pull/18) at portfolio closeout commit `e28ffa1`. CI run [29851083689](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29851083689) passes Rust, web, and API/PostGIS checks. The review documents and enforces trust boundaries, threat modeling, lifecycle/backup/incident controls, and ten owned findings; repository approval remains separate from FT-404 public-deployment approval.
- FT-402 is delivered through PR [#22](https://github.com/carlwelchdesign/flight-tracker-ai/pull/22) at final implementation commit `73e7157`. CI run [29856364366](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29856364366) passes Rust, web, and API/PostGIS checks. It proves visible source timeout/outage fallback, adversarial-input rejection, durable alert history across worker replacement, measured bounded/overflow backlog behavior, and an isolated logical PostGIS restore while preserving the honest FT-404 hosted-recovery boundary.
- FT-403 preparation is under review in PR [#23](https://github.com/carlwelchdesign/flight-tracker-ai/pull/23) at implementation commit `15227e4`. It adds a self-guided recruiter orientation, clarifies the outage control, and records the neutral-review protocol and current **Revise** decision without claiming independent participant evidence.
- FT-404 is active in draft PR [#24](https://github.com/carlwelchdesign/flight-tracker-ai/pull/24) at implementation commit `e33a21c`. Vercel project `flight-tracker-ai` is connected to this repository with `apps/web` as the Next.js root. CI run [29862882559](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29862882559) passes Rust, web, and API/PostGIS checks for the explicit free staging/production topology. The exact commit was also built as protected preview deployment `dpl_5jVqrumWgwHJiUBc4i4fHZkdo6LL` and production deployment `dpl_2hfw56Se2W9oSDx7fCQ4F3hHd2cb`.
- Vercel Marketplace resources `neon-bronze-curtain`, `neon-bistre-lantern`, and `clerk-celeste-door` are available. Production and staging use separate Neon Free projects in AWS `us-east-1`; PostGIS `3.5.0` is enabled. Clerk uses production keys for Production and test keys for Preview, its production domain is `flight-tracker-ai-one.vercel.app`, Organizations with required membership are enabled, and organization `Flight Tracker Portfolio` exists. Reviewer enrollment and authenticated hosted smoke remain pending.
- Vercel has `AUTH_MODE`, `OPERATIONS_MODE`, `INTERNAL_AUTH_KEY_ID`, `AUTH_ASSERTION_ISSUER`, and `AUTH_ASSERTION_AUDIENCE` configured for Development, Preview, and Production. Preview and Production now use distinct Render `API_BASE_URL` and `INTERNAL_AUTH_SECRET` values.
- The first public-alias observation showed the original deployment still defaulting to development auth and revealed that noninteractive Vercel input had stored the five non-secret runtime settings as `[SENSITIVE]` in Production and Preview. The settings are corrected and verified. Commit `790e022` fixes the later blank Clerk form under strict CSP; commit `63e3bbd` keeps sign-up on the application domain. CI runs [29867545134](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29867545134) and [29868085207](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29868085207) pass all jobs. Commit `bcf49cc` replaces the root sign-in wall with the read-only public flight tracker and passes CI run [29868610558](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29868610558). Production deployment `dpl_5CvqF2Dbg6LnwZkDc8ccRZnutS4e` now shows the fleet map, three-flight board, NOAA weather context, and selectable details immediately, while `/sign-in`, `/sign-up`, operational actions, and backend routes preserve their intended protection. The live sanitized verifier passes this production web/API boundary and rejects the former sign-in-only landing.
- Render Blueprint `exs-d9ft018okrbs738q5r60` created free production service `srv-d9ft2gn7f7vs739ass40` and staging service `srv-d9ft2gn7f7vs739ass3g` in Virginia. Each uses its own Neon project and assertion secret. A sanitized hosted probe found the otherwise-idle alert projection becoming stale; commits `a665494` and `6a4929e` add a periodic supervised heartbeat and API HSTS. CI run [29865574640](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29865574640) passes all jobs. The final promotion commit will switch both services to `main` after hosted smoke.
- Render deploys `dep-d9ftbi61a83c7396hbsg` (production) and `dep-d9ftf2naqgkc738okfig` (staging) run commit `6a4929e`. Both pass health, readiness, PostGIS/migration, HSTS, distinct assertion-secret, and four-worker checks. The sanitized public verifier passes against protected preview `dpl_FNbngNWmbKNSafY5rvNypHjPaPzS` and publication-ready production `dpl_FXv3uAUVCKCRTTfTm5xRj7rn1pWE`; temporary verifier identities were removed afterward.
- Neon retained the manual production snapshot `main at 2026-07-21 20:46:51 UTC (manual)`. A separate 13:45 PDT point-in-time restore matched production with PostGIS `3.5.0`, 14 successful migrations, one operator, and zero identity, membership, alert, and action rows. An earlier pre-migration restore correctly failed closed. Both temporary restore branches were deleted after verification, leaving production `main` unchanged.
- F401-004 is closed at implementation commit `e9e5f76`: operator-scoped membership foreign keys now protect both current alert assignments and assignment audit rows. CI run [29833385671](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29833385671) proves direct-database and authenticated-API cross-tenant rejection plus valid same-tenant assignment.
- Browser policy implementation commit `dc08690` adds strict nonce-aware Clerk CSP and production response hardening. CI run [29833848250](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29833848250), the standalone header smoke, and all 30 web tests pass; F401-005 is controlled by FT-404's pre-publication hosted-Clerk smoke.
- F401-010 is closed at implementation commit `38cf7b7`: public health/readiness probes now expose one status field, while detailed worker/database/PostGIS diagnostics require authorization. CI run [29834083229](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834083229) verifies the public, unauthorized, authenticated, PostGIS, BFF, and console contracts.
- Assertion rotation implementation commit `5dca15e` and [`CREDENTIAL_ROTATION_RUNBOOK.md`](CREDENTIAL_ROTATION_RUNBOOK.md) define named active/previous keys, zero-downtime and emergency sequences, rollback, and evidence. CI run [29834813131](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29834813131) verifies overlap/retirement, cross-language API authentication, 32 web tests, 77 Rust tests, and the browser-asset secret scan. F401-001 is controlled by FT-404's environment-secret gate.
- Retention implementation commit `fdf75af` extends the two-person preview/approval/execution workflow to authorization audit, expired session revocations, and exclusive inactive identity minimization. CI run [29838690551](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29838690551) proves tenant scope, active/shared identity preservation, typed tombstones, and simulated restore suppression; F401-002 is controlled by FT-404's hosted scheduling gate.
- Terminal alert retention commit `b937f8a` deletes only whole old dismissed/resolved series, orders action/evidence deletion before alerts, preserves recent/open/mixed/cross-tenant series, suppresses exact logical replay, and permits new material evidence at the next revision. CI run [29839592816](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29839592816) passes all three jobs.
- Normalized-fact retention commit `420f145` deletes provider-scoped old observations, whole unreferenced terminal flight aggregates, and whole unreferenced expired hazard series in dependency order. CI run [29840281133](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29840281133) proves current/active/referenced/provider-mismatched/cross-tenant preservation and exact restore suppression.
- Scheduled-retention commit `5c320b8` adds separately approved exact-policy schedules, durable attempt/failure evidence, drift-free idempotent slots, automatic retirement with policy replacement, and a supervised Rust worker. CI run [29841291067](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29841291067) passes all three jobs including automatic PostGIS deletion and inactive-actor fail-closed behavior.
- Restore-integrity commit `ec9ea2a` adds administrator-only tenant resurrection diagnostics across every tombstone class plus a controlled backup-restore runbook. CI run [29841960517](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29841960517) passes all three jobs and proves healthy lifecycle state, deliberate conflict detection, and cross-tenant isolation. Managed backup configuration and the recorded drill remain open.
- Exact-inventory commit `a521e91` stores an exact-key SHA-256 fingerprint for every new manual and scheduled retention run and executes under a repeatable-read snapshot. CI run [29843308802](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29843308802) passes all three jobs and proves that same-count record substitution fails closed without deletion.
- Bounded-write commit `cd08649` enforces matching Rust/PostgreSQL limits for dispatcher notes, action identifiers, and session-revocation reasons, normalizes idempotency, and surfaces the note limit in the console. CI run [29844162906](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29844162906) passes all three jobs including API and direct-database rejection.
- Sensitive-write monitoring commit `d975ac3` detects controlled credential/email patterns in dispatcher comments and session-revocation reasons while returning only redacted tenant-scoped signals. CI run [29845085036](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29845085036) passes all three jobs and proves detection, non-leakage, ordinary-text rejection, and cross-tenant isolation against fresh PostGIS. F401-007 is controlled by FT-404's hosted verifier gate.
- Hosted-verifier commits `943bb65` and `f3d3e08` add sanitized fail-closed checks for administrator audit/export/monitoring/integrity access, viewer/operator denial, marker redaction, expected signal severities, cross-tenant exclusion, and exact retention disposition counts. CI run [29846123252](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29846123252) passes the ten-case verifier suite and all Rust, web, and PostGIS jobs. FT-404 must execute it against the chosen hosted environment before publication.
- FT-005 is delivered through PR [#19](https://github.com/carlwelchdesign/flight-tracker-ai/pull/19) at scope commit `7b052f9` and delivery-record commit `d1b9f46`. CI run [29847937301](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29847937301) passes Rust, web, and API/PostGIS checks. The active roadmap now targets a non-commercial recruiter/hiring-manager portfolio, with free best-effort positions or replay instead of paid-provider procurement.
- The FT-005 rebaseline makes `validate_ft301_evidence.py --require-complete`, commercial contract evidence, priced trials, an SLA, and real-operator approval irrelevant to FT-401 completion. `validate_ft401_review.py --require-complete` now passes with all critical/high findings closed and residual medium risks assigned to expiring downstream gates.
- The MVP should work with deterministic simulated flights before relying on a paid data feed.
- NOAA Aviation Weather is approved as the first live integration target, with explicit source-age and degraded-state handling.
- OpenSky must not be integrated into the automated or commercial product without a written operational/commercial license.
- Cirium Sky Stream and FlightAware Firehose remain optional future commercial candidates; no paid provider is required for the portfolio release.
- FlightAware AeroAPI must not be used for the dispatcher display under its published self-service license because that license excludes commercial aircraft situational displays.
- FAA SCDS/SWIFT and NMS remain separately access-gated and must not be treated as sole operational sources.
- Do not begin ACARS integration or flight optimization until the gates in M5 are satisfied.

## Update protocol

When work begins, replace `Active ticket: None` with the ticket ID, branch name, owner, and a one-sentence next action. When a ticket completes, update its checklist, final commit SHA, PR URL, the progress table, milestone checklist if applicable, last-updated date, and handoff notes.
