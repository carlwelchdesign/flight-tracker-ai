# FT-423 — Add a portfolio live-position provider failover

Status: In progress

Branch: `fix/ft-423-aircraft-provider-failover`
Final implementation commit: `af0f4a0`
Pull request: [#66](https://github.com/carlwelchdesign/flight-tracker-ai/pull/66)
Owner: Backend and frontend engineering

Keep the public tracker useful during an ADSB.lol outage by attempting the
compatible Airplanes.live regional endpoint only after the primary request
fails. Preserve deterministic replay as the guaranteed provider-independent
experience and make the actual source visible wherever live data is shown.

Dependencies: FT-405, FT-410

## Acceptance checklist

- [x] ADSB.lol remains the primary source and a successful primary response
      never calls Airplanes.live.
- [x] Airplanes.live is attempted only after primary failure and a process-wide
      limiter permits no more than one fallback request per second, including
      retries across all regional workers.
- [x] The accepted provider is propagated through normalized events, public
      aircraft records, source status, logs, and provider-specific attribution.
- [x] Both provider paths remain bounded, ephemeral, uncached, unpersisted,
      unavailable to exports and LLMs, and limited to the existing region
      catalog.
- [x] When both providers fail, the UI reports live unavailability truthfully,
      retains the last accepted picture when available, and keeps replay usable.
- [x] Public and protected source UI uses provider-neutral language and shows
      the actual provider and applicable terms.
- [x] Focused primary-success, fallback-success, both-fail, rate-limit, source
      propagation, and UI tests pass.
- [x] Rust formatting, Clippy, Rust and web tests, lint, typecheck, production
      build, browser verification, hosted smoke checks, and diff hygiene pass.
- [ ] Branch, final commit, pull request, required checks, rollout evidence, and
      handoff notes are recorded before completion.

## Portfolio-only constraint

Airplanes.live documents a no-SLA, non-commercial API with a one-request-per-
second limit, but its public materials do not provide the same precise data
license as ADSB.lol's ODbL grant. This ticket is an owner-approved exception for
the public non-commercial portfolio demonstration only. It permits ephemeral
display with attribution and no persistence, cache, export, redistribution, or
LLM use. Commercial or operational use requires a new rights review and written
permission where necessary.

## Non-goals

- Treating either free source as complete, authoritative, operational, or
  covered by an SLA.
- Adding OpenSky or a commercial provider.
- Persisting provider positions or building provider-derived routes, schedules,
  delays, alerts, or recommendations.
- Replacing deterministic replay as the reliable demonstration path.

## Verification evidence

- The Rust suite passes 138 library tests, 14 configuration tests, all six
  integration/golden/schema tests, and strict workspace Clippy and formatting.
- The web suite passes all 154 tests across 47 files, ESLint, TypeScript, the
  Next.js production build, and a zero-finding production dependency audit.
- Focused tests prove primary short-circuiting, fallback success, explicit
  all-provider failure, shared request spacing across cloned clients, canonical
  and public source propagation, dynamic provider filtering, and linked public
  and protected fallback attribution.
- `docker compose config --quiet` and `git diff --check` pass.
- Direct SFO compatibility probe on 2026-07-22 returned HTTP 200 from both
  ADSB.lol (145 aircraft) and Airplanes.live (146 aircraft). Production will
  therefore continue using the primary until a request fails.
- Implementation commit `af0f4a0` is pushed and PR
  [#66](https://github.com/carlwelchdesign/flight-tracker-ai/pull/66) is open.
  Hosted deployment, browser smoke, and required-check evidence are pending.
