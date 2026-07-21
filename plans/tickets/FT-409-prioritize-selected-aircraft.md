# FT-409 — Prioritize selected aircraft details

Status: In review

Branch: `feat/ft-409-prioritize-selected-aircraft`
Latest implementation commit: `4b37eef`
Final commit: Pending
Pull request: [#29](https://github.com/carlwelchdesign/flight-tracker-ai/pull/29)
Owner: Frontend product engineering

## Outcome

Place the selected-aircraft evidence panel above the current aircraft list in
the tracker sidebar. The map remains the dominant workspace, while the detail a
viewer explicitly selected is visible before the longer regional traffic list.

Dependency: FT-405 live navigable tracker.

## Acceptance checklist

- [x] Selected-aircraft evidence precedes the current-picture aircraft list in
      visual and document order.
- [x] Map, aircraft selection, skip link, scroll behavior, and mobile stacking
      continue to work.
- [x] A behavior test protects the intended panel order.
- [x] Focused tests, typecheck, lint, production build, and browser verification
      pass.
- [ ] The dedicated branch has an intentional commit, pull request, passing
      checks, and updated ticket evidence.

## Verification evidence

- The focused tracker suite passes all three scenarios, including an explicit
  document-order assertion for selected details before the aircraft list.
- TypeScript, ESLint, and the Next.js production build pass on Node 20.20.1.
- Browser verification against the production Rust APIs confirms the selected
  panel starts at the top of the sidebar, the scrollable traffic list begins
  directly beneath it, map and NOAA layers remain visible, horizontal overflow
  is absent, and no application errors or framework overlay are present.

## Non-goals

- Do not change live-traffic coverage, provider cadence, weather data, or map
  visualization behavior.
- Do not add atmospheric raster or animated wind layers in this layout ticket.
