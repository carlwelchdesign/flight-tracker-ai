# FT-426 — Replace internal route-assessment language

Status: Complete

Branch: `fix/ft-426-route-risk-copy`
Final implementation commit: `6e48fd0`
Pull request: [#72](https://github.com/carlwelchdesign/flight-tracker-ai/pull/72)
Owner: Frontend engineering and content design

Replace the aircraft inspector's internal “Decision intelligence / Not
evaluated” terminology with a direct explanation of why route risk is
unavailable. Preserve the underlying eligibility rules and evidence boundary.

Dependencies: FT-413, FT-425

## Acceptance checklist

- [x] Live aircraft without route evidence display “Route risk unavailable.”
- [x] Live copy explains that the traffic feed lacks the route information
      needed to evaluate weather conflicts.
- [x] Replay aircraft with incomplete route evidence use the same plain heading
      and clearly identify incomplete evidence.
- [x] “Decision intelligence” and “Not evaluated” no longer appear in the
      public aircraft inspector.
- [x] Evaluation rules, data requirements, API behavior, and available replay
      attention results remain unchanged.
- [x] Focused tests, full web tests, lint, typecheck, production build, browser
      verification, and diff hygiene pass.
- [x] Branch, commits, pull request, required checks, and hosted evidence are
      recorded before completion.

## Non-goals

- Adding route-risk evaluation to live ADS-B traffic.
- Changing the replay attention policy or backend response contract.
- Removing the truthful unavailable state.

## Verification evidence

- Focused component suite: 13/13 tests passed.
- Full web suite: 47 files and 154 tests passed.
- `npm run lint`, `npm run typecheck`, and `npm run build` passed on Node
  24.18.0.
- Local browser verification confirmed the aircraft inspector renders without
  the retired “Decision intelligence” and “Not evaluated” labels. Exact live
  and replay unavailable states are covered by component tests.
- `git diff --check` passed.
- PR #72 merged as `bb4ddfc26e60a00eb68cad3396a7f969a5db76f4`.
- Main CI run `29960739243` passed its Rust checks, full web checks, and API /
  PostGIS smoke suite.
- Production deployment `dpl_4UPAijqcBMwu3Q4oowV6J3HG3Bw8` is Ready and
  assigned to <https://flight-tracker-ai-one.vercel.app/>.
- Hosted browser verification opened a live aircraft's details and confirmed
  “Route risk unavailable,” “Position data only,” and the missing-route
  explanation are visible while the retired labels are absent.
- The public URL returned HTTP 200 and the deployment had no error-level runtime
  logs during verification.
