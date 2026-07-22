# FT-422 — Make map information panels movable and recoverable

Status: In progress

Branch: `fix/ft-422-draggable-map-panels`
Final commit: Pending
Pull request: Pending
Owner: Frontend engineering and product design

Prevent the airport forecast/PIREP panel and NOAA layer controls from obscuring
one another. Treat both as user-controlled map panels that can be brought
forward, moved, closed completely, and reopened from one discoverable menu.

Dependencies: FT-411, FT-416

## Acceptance checklist

- [ ] A Panels menu exposes independent controls for the NOAA layers and the
      current regional airport forecast/PIREP panel.
- [ ] Each visible desktop panel has an accessible drag handle and close button;
      dragging remains bounded within the map stage.
- [ ] Clicking or focusing a panel brings it in front of the other panel.
- [ ] Closing a panel removes it completely, and the Panels menu can reopen it
      without reloading the tracker or losing already loaded panel data.
- [ ] On narrow viewports the panels return to document flow, remain closable,
      and cannot obscure one another.
- [ ] Keyboard and screen-reader users can discover panel visibility, close and
      reopen panels, and use all panel contents without requiring drag.
- [ ] Focused component tests, the full web test suite, lint, typecheck,
      production build, browser verification, and diff hygiene pass.
- [ ] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Persisting panel positions between devices or browser sessions.
- Changing weather, TAF, PIREP, aircraft, replay, or map data contracts.
- Adding window resizing or introducing a general desktop-window framework.

## Verification evidence

Pending implementation and hosted verification.
