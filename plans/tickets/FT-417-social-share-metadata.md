# FT-417 — Add branded social-share metadata

Status: In progress

Branch: `feat/ft-417-social-share-metadata`
Final commit: Pending
Pull request: Pending
Owner: Frontend product engineering

Make the public tracker produce an accurate, branded link preview instead of
the hosting platform's generic preview when its production URL is shared.

Dependencies: FT-405

## Acceptance checklist

- [ ] The root page publishes a stable title, useful description, application
      name, canonical production URL, and index/follow policy.
- [ ] Open Graph metadata identifies the public tracker as a website and uses a
      1200 by 630 branded preview image with descriptive alternative text.
- [ ] Twitter/X metadata requests a large-image card and uses the same truthful
      title, description, and branded image.
- [ ] The preview copy describes live regional aircraft, deterministic replay,
      aviation weather, trajectories, and explainable attention without
      implying certified or operational use.
- [ ] Metadata configuration is isolated from the layout and covered by a
      focused contract test.
- [ ] Lint, TypeScript, focused tests, the full web suite, and the production
      build pass.
- [ ] Rendered HTML and the generated image route are verified locally and on
      the hosted production URL.
- [ ] Branch, commit, PR, required checks, merge, and production evidence are
      recorded before completion.

## Non-goals

- Per-aircraft dynamic share images.
- Persisting share requests or introducing analytics.
- Changing public tracker behavior, data sources, or operational boundaries.

## Verification evidence

Pending.
