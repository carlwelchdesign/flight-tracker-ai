# FT-422 — Make map information panels movable and recoverable

Status: Complete

Branch: `fix/ft-422-draggable-map-panels`
Final commit: `3485e3f`
Pull request: [#63](https://github.com/carlwelchdesign/flight-tracker-ai/pull/63)
Owner: Frontend engineering and product design

Prevent the airport forecast/PIREP panel and NOAA layer controls from obscuring
one another. Treat both as user-controlled map panels that can be brought
forward, moved, closed completely, and reopened from one discoverable menu.

Dependencies: FT-411, FT-416

## Acceptance checklist

- [x] A Panels menu exposes independent controls for the NOAA layers and the
      current regional airport forecast/PIREP panel.
- [x] Each visible desktop panel has an accessible drag handle and close button;
      dragging remains bounded within the map stage.
- [x] Clicking or focusing a panel brings it in front of the other panel.
- [x] Closing a panel removes it completely, and the Panels menu can reopen it
      without reloading the tracker or losing already loaded panel data.
- [x] On narrow viewports the panels return to document flow, remain closable,
      and cannot obscure one another.
- [x] Keyboard and screen-reader users can discover panel visibility, close and
      reopen panels, and use all panel contents without requiring drag.
- [x] Focused component tests, the full web test suite, lint, typecheck,
      production build, browser verification, and diff hygiene pass.
- [x] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Persisting panel positions between devices or browser sessions.
- Changing weather, TAF, PIREP, aircraft, replay, or map data contracts.
- Adding window resizing or introducing a general desktop-window framework.

## Verification evidence

- `MapFloatingPanel` keeps movement, focus stacking, close behavior, responsive
  drag disabling, and pointer/keyboard bounds in one reusable client component.
  The weather and airport-intelligence components retain their existing data
  loading and presentation responsibilities.
- Seven focused floating-panel and map lifecycle tests pass. They cover bounded
  pointer movement, keyboard reset, narrow-screen drag disabling, focus-based
  stacking, complete close, menu reopening, and preserved mounted content.
- All 147 web tests across 44 files pass, along with ESLint, TypeScript, the
  Next.js production build, and `git diff --check` under Node.js 24.18.0.
- Desktop browser verification found zero initial panel overlap. Pointer drag
  changed the weather panel transform, clicking the expanded airport panel
  raised it, closing weather moved focus to Panels, and reopening retained the
  moved position and panel content. No application error overlay or console
  error appeared.
- At 390 by 844, both panels rendered in document flow with zero overlap,
  dragging disabled, no horizontal overflow, and a 390-pixel document width.
- GitHub Actions run
  [29944316820](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29944316820)
  passed Rust, web, and API/PostGIS checks. Vercel preview deployment
  `flight-tracker-64m6dzkr3-carlwelchdesigns-projects.vercel.app` completed
  successfully.
