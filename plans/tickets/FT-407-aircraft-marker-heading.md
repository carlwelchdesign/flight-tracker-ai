# FT-407 — Align live aircraft markers with trajectories

Status: Complete

Branch: `fix/ft-407-aircraft-marker-heading`
Latest implementation commit: `c36f763`
Final implementation commit: `c36f763`
Pull request: [#27](https://github.com/carlwelchdesign/flight-tracker-ai/pull/27)
Owner: Frontend product engineering

## Outcome

Make every aircraft icon on the public live MapLibre tracker point in the same
direction as its supplied true heading and estimated trajectory. Correct the
live marker glyph's visual axis by negative 90 degrees without changing source
heading facts, trajectory math, or the separate north-facing SVG used by the
protected operations console.

Dependency: FT-405 live navigable tracker and FT-406 flight trajectories. This
ticket is intentionally stacked on the active FT-405 feature branch.

## Acceptance checklist

- [x] The public live-map marker applies a negative 90-degree visual offset to
      every supplied aircraft heading.
- [x] North, east, south, and west headings map deterministically to the
      corrected icon rotation.
- [x] Missing heading remains deterministic and does not fabricate a motion
      fact in the evidence panel.
- [x] The correction changes presentation only; source heading values and
      trajectory calculations remain unchanged.
- [x] Focused tests, full web tests, typecheck, lint, production build, and
      whitespace checks pass.
- [x] Browser verification confirms selected live markers align with their
      dashed projections on desktop and remain usable on mobile.
- [x] The dedicated branch has intentional commits, a stacked pull request,
      passing required checks, and updated ticket/status evidence.

## Implementation notes

- Keep the glyph-axis correction explicit at the public marker presentation
  boundary.
- Do not apply the offset to the protected console's north-facing SVG glyph.
- Do not alter the ADS-B heading contract or geodesic projection policy.

## Verification evidence

- Five focused tests prove the negative 90-degree mapping for north, east,
  south, west, and missing-heading presentation.
- The complete web suite passes with 73 tests across 23 files. TypeScript,
  ESLint, `next build`, and `git diff --check` also pass on Node `20.20.1`.
- A local production build against the live staging Rust API verified that a
  selected aircraft with a supplied `298.26`-degree true heading renders the
  public marker at `208.26` degrees while retaining the original heading in the
  evidence panel and trajectory calculation.
- Mobile browser verification at `390x844` rendered 144 corrected markers with
  zero horizontal overflow.
- CI run [29875663012](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29875663012)
  passes Rust, web, API/PostGIS, Vercel, and Vercel Preview Comments for commit
  `c36f763`.
