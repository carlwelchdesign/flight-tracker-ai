# FT-421 — Align replay aircraft motion and heading

Status: Complete

Branch: `fix/ft-421-replay-heading-alignment`
Final commit: `2bd1349`
Pull request: [#62](https://github.com/carlwelchdesign/flight-tracker-ai/pull/62)
Owner: Frontend and replay engineering

Correct the deterministic FT303 scenario path so its observed trail moves in
the same northwest direction as its recorded true heading and public aircraft
glyph. Preserve the existing live-marker axis correction and operational rule
behavior.

Dependencies: FT-407, FT-414

## Acceptance checklist

- [x] FT303's consecutive replay positions align with its supplied 315-degree
      true heading instead of drawing a contradictory northeast trail.
- [x] The public replay marker uses the existing negative 90-degree glyph-axis
      correction; live aircraft presentation is unchanged.
- [x] Regression coverage proves the scenario's segment bearings remain within
      a small tolerance of their recorded headings.
- [x] Replay attention, timeline interpolation, telemetry, and deterministic
      scenario behavior continue to pass.
- [x] Focused tests, full Rust and web tests, static checks, production builds,
      browser verification, and diff hygiene pass.
- [x] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Inferring headings from live provider positions.
- Changing the global marker transform, projection mathematics, attention
  policy, or planned route.

## Verification evidence

- FT303's two consecutive replay segments now have initial great-circle bearings
  of approximately 315.79 and 315.23 degrees. The regression limit is five
  degrees from the supplied segment heading.
- The existing `liveMarkerRotationDegrees(315)` presentation remains 225
  degrees. No marker transform, live position, attention policy, planned route,
  or current 60-second attention position changed.
- Four focused Rust public replay tests and 15 focused web replay/public tracker
  tests pass. Strict Clippy, Rust formatting, all 131 Rust library tests plus
  binary/integration/golden/schema tests, all 143 web tests, ESLint, TypeScript,
  and the Next.js production build pass.
- A browser run against the corrected public timeline at 60 seconds showed
  FT303's two-point trail extending southeast behind the northwest-facing glyph,
  with the marker retaining the tested 225-degree CSS presentation value.
- `git diff --check` passes.
- GitHub Actions run
  [29942050285](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29942050285)
  passed Rust, web, and API/PostGIS checks. The Vercel preview deployment also
  completed successfully.
