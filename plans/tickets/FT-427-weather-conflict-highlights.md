# FT-427 — Highlight evaluated weather conflicts

Status: Complete

Branch: `feat/ft-427-weather-conflict-highlights`
Final implementation commit: `b221661`
Pull request: [#74](https://github.com/carlwelchdesign/flight-tracker-ai/pull/74)
Owner: Frontend engineering and product design

Make aircraft with deterministic route-and-weather conflict evidence visibly
distinct on the public map and aircraft list. Preserve the difference between
replay evidence and live position-only traffic.

Dependencies: FT-413, FT-426

## Acceptance checklist

- [x] Replay aircraft with a deterministic conflict result receive a distinct
      map highlight and a “Weather conflict” aircraft-list badge.
- [x] Highlight eligibility comes only from a validated `requires_attention`
      result effective at the current replay time.
- [x] Highlighted map markers expose the conflict state in their accessible
      label.
- [x] Live and stale ADS-B aircraft never receive the conflict highlight because
      the public feed does not include route evidence.
- [x] A concise replay legend explains the highlight without implying an
      operational or live assessment.
- [x] Focused tests, full web tests, lint, typecheck, production build, browser
      verification, and diff hygiene pass.
- [x] Branch, commits, pull request, required checks, and hosted evidence are
      recorded before completion.

## Non-goals

- Inferring or purchasing filed routes for live aircraft.
- Correlating a motion projection with weather and calling it a filed route.
- Changing Rust evaluation policy, scoring, or the replay attention contract.

## Verification evidence

- Focused map and public-tracker suites: 18/18 tests passed.
- Full web suite: 47 files and 155 tests passed.
- `npm run lint`, `npm run typecheck`, and `npm run build` passed on Node
  24.18.0.
- Tests prove the highlight is absent from live traffic, appears only after the
  replay evaluation becomes effective, clears when the time machine rewinds,
  and updates the map marker's accessible label.
- `git diff --check` passed.
- PR #74 passed its Rust, web, API/PostGIS, and Vercel checks and merged as
  `5b9eb86f19eb9b7ae982a3543990016fc751417c`.
- Production deployment `dpl_8xAatBpWrdEyxUuo4nBNDjTbPsG2` is Ready and
  assigned to <https://flight-tracker-ai-one.vercel.app/>.
- Hosted browser verification at replay time 60 seconds confirmed FT303's
  highlighted map marker, “Weather conflict” list badge, replay-evidence
  legend, accessible marker label, and critical evidence panel. Rewinding to
  zero removed the highlight, and live traffic showed no conflict badge or
  conflict marker.
- The hosted replay URL returned HTTP 200 and the deployment had no error-level
  runtime logs during verification.
