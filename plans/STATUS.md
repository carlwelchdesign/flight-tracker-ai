# Project Status

Last updated: 2026-07-21

## Current state

- Current milestone: M4 — Portfolio launch and demonstration hardening
- Active ticket: FT-404 — Deploy the public portfolio and preview environments
- Branch: `feat/ft-404-production-deployment`
- Pull request: [#24](https://github.com/carlwelchdesign/flight-tracker-ai/pull/24)
- Owner: Platform, backend, security, and full-stack engineering
- Overall status: M0, M1, M2, and M3 are complete; M4 is 2/4 complete, with recruiter-demo validation and public portfolio deployment still explicit gates
- Next action: Enroll the reviewer in the production Clerk organization and run
  the authenticated browser and hosted FT-401 smoke before final promotion.

## Milestone checklist

- [x] M0 — Foundation, feasibility, and portfolio rebaseline
- [x] M1 — Simulated operations console
- [x] M2 — Live weather and hazard intelligence
- [x] M3 — Portfolio live data and operational workflow
- [ ] M4 — Portfolio launch and demonstration hardening
- [ ] M5 — Optimization research and controlled recommendations

## Current product correction

- FT-405 is active on `feat/ft-405-live-navigable-tracker`. The existing public
  SVG/replay surface is an interim prototype, not the intended end state. The
  next release must expose the existing bounded ADSB.lol Rust adapter through a
  sanitized public no-store read model, replace the fixed SVG with MapLibre,
  animate aircraft between 30-second best-effort snapshots, and retain replay
  only as an explicit degraded-source fallback. See
  [`FT-405-live-navigable-tracker.md`](tickets/FT-405-live-navigable-tracker.md).

## Ticket progress

| Milestone | Complete | Total |
| --- | ---: | ---: |
| M0 | 5 | 5 |
| M1 | 4 | 4 |
| M2 | 4 | 4 |
| M3 | 4 | 4 |
| M4 | 2 | 4 |
| M5 | 0 | 3 |

## Handoff notes

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
