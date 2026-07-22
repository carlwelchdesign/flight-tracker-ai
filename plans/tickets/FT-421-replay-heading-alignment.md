# FT-421 — Align replay aircraft motion and heading

Status: In progress

Branch: `fix/ft-421-replay-heading-alignment`
Final commit: Pending
Pull request: Pending
Owner: Frontend and replay engineering

Correct the deterministic FT303 scenario path so its observed trail moves in
the same northwest direction as its recorded true heading and public aircraft
glyph. Preserve the existing live-marker axis correction and operational rule
behavior.

Dependencies: FT-407, FT-414

## Acceptance checklist

- [ ] FT303's consecutive replay positions align with its supplied 315-degree
      true heading instead of drawing a contradictory northeast trail.
- [ ] The public replay marker uses the existing negative 90-degree glyph-axis
      correction; live aircraft presentation is unchanged.
- [ ] Regression coverage proves the scenario's segment bearings remain within
      a small tolerance of their recorded headings.
- [ ] Replay attention, timeline interpolation, telemetry, and deterministic
      scenario behavior continue to pass.
- [ ] Focused tests, full Rust and web tests, static checks, production builds,
      browser verification, and diff hygiene pass.
- [ ] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Inferring headings from live provider positions.
- Changing the global marker transform, projection mathematics, attention
  policy, or planned route.

## Verification evidence

Pending.
