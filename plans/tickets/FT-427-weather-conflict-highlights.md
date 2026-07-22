# FT-427 — Highlight evaluated weather conflicts

Status: In progress

Branch: `feat/ft-427-weather-conflict-highlights`
Final implementation commit: Pending
Pull request: Pending
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
- [ ] Focused tests, full web tests, lint, typecheck, production build, browser
      verification, and diff hygiene pass.
- [ ] Branch, commits, pull request, required checks, and hosted evidence are
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
- `git diff --check` passed. Preview and hosted browser evidence remain pending.
