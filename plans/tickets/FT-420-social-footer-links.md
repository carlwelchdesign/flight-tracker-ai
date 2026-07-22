# FT-420 — Add portfolio social links to the public footer

Status: Complete

Branch: `feat/ft-420-social-footer-links`
Final commit: `5cba771`
Pull request: [#61](https://github.com/carlwelchdesign/flight-tracker-ai/pull/61)
Owner: Frontend engineering

Add accessible LinkedIn and GitHub SVG links at the bottom of the public flight
tracker so recruiters can move directly from the product to the creator's
profile or project source.

Dependencies: FT-419

## Acceptance checklist

- [x] The public page footer includes recognizable LinkedIn and GitHub SVG
      marks without adding an icon dependency.
- [x] LinkedIn opens Carl Welch's profile and GitHub opens this project's
      repository in a new tab.
- [x] Both links have accessible names, visible hover/focus treatment, and safe
      external-link attributes.
- [x] The footer remains usable at desktop and mobile widths.
- [x] Focused tests, lint, type checking, production build, runtime browser
      verification, and diff hygiene pass.
- [x] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Adding an icon library, analytics, contact forms, or additional social links.
- Changing flight-tracker behavior, data sources, or page metadata.

## Verification evidence

- The focused public tracker test passes all 12 cases, including exact link
  destinations, accessible names, SVG treatment, and external-link attributes.
- Web lint and TypeScript pass. The complete web suite passes all 143 tests
  across 43 files, and the production build completes successfully.
- Local browser verification at 1440 by 1000 and 390 by 844 showed both footer
  marks. The mobile document measured `scrollWidth === clientWidth` at 390
  pixels, and both DOM destinations matched the accepted URLs.
- `git diff --check` passes. The change adds no package dependency or generated
  asset.
- GitHub Actions run
  [29941214012](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29941214012)
  passed Rust, web, and API/PostGIS checks. Vercel preview deployment
  `CvGjWKk5CrbvvxpjK38sNrj5wDZX` completed successfully; its generated preview
  URL remains access-protected by the project's Vercel deployment policy.
