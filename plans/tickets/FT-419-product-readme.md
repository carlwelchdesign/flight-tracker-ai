# FT-419 — Refresh the product README and screenshots

Status: Complete

Branch: `docs/ft-419-product-readme`
Final commit: `0e4c6f1`
Pull request: [#60](https://github.com/carlwelchdesign/flight-tracker-ai/pull/60)
Owner: Product documentation

Replace the setup-first repository README with a recruiter-friendly overview of
the public flight-tracker experience, grounded in the features that are live in
production. Capture current production screenshots so the repository presents
the product visually without overstating its operational or AI capabilities.

Dependencies: FT-413, FT-414, FT-415, FT-416, FT-417, FT-418

## Acceptance checklist

- [x] The README leads with the product outcome and links to the live public
      tracker.
- [x] Current public features are described accurately, including regional live
      traffic, trajectories, atmospheric layers, attention explanation, replay
      telemetry, search/share state, and airport intelligence.
- [x] The architecture and data-source boundaries distinguish live,
      deterministic, optional, and in-development capabilities.
- [x] Local setup, verification, repository structure, and deployment guidance
      remain available without dominating the product overview.
- [x] Current desktop and mobile production screenshots are stored in the
      repository and render from the README.
- [x] Documentation and image paths pass diff-hygiene and link checks.
- [x] Branch, final commit, pull request, required checks, and visual evidence
      are recorded before completion.

## Non-goals

- Changing application behavior, production data, infrastructure, or auth.
- Advertising the optional OpenAI drafting adapter as a public product feature.
- Replacing FT-403 neutral recruiter validation or FT-502 aviation-domain review.

## Verification evidence

- Production replay was exercised without authentication at 1440 by 1000 and
  390 by 844. The browser reported no application errors during capture.
- `docs/images/flight-tracker-desktop.png` is a 1440 by 1000 PNG showing the
  replay map, weather controls, selected FT303 attention explanation, aircraft
  list, and telemetry. `docs/images/flight-tracker-mobile.png` is a 390 by 844
  PNG showing the responsive replay controls and telemetry.
- All relative Markdown and HTML image targets in `README.md` resolve to tracked
  files. The production root returned HTTP 200.
- Prettier passed for the README and ticket files; `git diff --check` passed.
- GitHub Actions run
  [29940600554](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29940600554)
  passed Rust, web, and API/PostGIS checks. The Vercel preview deployment also
  completed successfully.
