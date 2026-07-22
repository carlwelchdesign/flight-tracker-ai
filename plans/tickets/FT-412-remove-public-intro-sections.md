# FT-412 — Remove public intro sections

Status: In progress

Branch: `fix/ft-412-remove-top-intro-sections`
Latest implementation commit: Pending
Final implementation commit: Pending
Pull request: Pending
Owner: Frontend product engineering

## Outcome

Make the public flight tracker begin directly with its product header and live
controls by removing the global evaluation-environment banner and the public
recruiter walkthrough section.

The protected operations console may retain its own orientation content. The
keyboard-only skip link remains because it is an accessibility control rather
than a visible introductory section.

## Acceptance checklist

- [x] The public root no longer renders the evaluation-environment banner.
- [x] The public root no longer renders the recruiter walkthrough section.
- [x] The public root begins with the Flight Tracker AI header and retains live
      traffic, weather, map, aircraft selection, and secure-console controls.
- [x] Removed banner code, styles, tests, and banner-only context helpers do not
      leave dead production paths.
- [ ] Focused tests, typecheck, lint, production build, and browser verification
      pass.
- [ ] Branch, commits, PR, passing checks, and delivery evidence are recorded.

## Non-goals

- Removing the protected console's orientation content.
- Removing the keyboard skip link, source attribution, freshness states, or the
  non-operational language attached to weather products.

## Current verification evidence

- The complete web suite passes 104 tests across 31 files. The public tracker
  regression proves both requested sections are absent while the product header,
  regional traffic, aircraft details, and protected-console link remain.
- Typecheck, zero-warning lint, and the production Next.js build pass.
- Hosted preview browser verification remains pending publication.
