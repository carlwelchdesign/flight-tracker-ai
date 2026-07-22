# FT-419 — Refresh the product README and screenshots

Status: In progress

Branch: `docs/ft-419-product-readme`
Final commit: Pending
Pull request: Pending
Owner: Product documentation

Replace the setup-first repository README with a recruiter-friendly overview of
the public flight-tracker experience, grounded in the features that are live in
production. Capture current production screenshots so the repository presents
the product visually without overstating its operational or AI capabilities.

Dependencies: FT-413, FT-414, FT-415, FT-416, FT-417, FT-418

## Acceptance checklist

- [ ] The README leads with the product outcome and links to the live public
      tracker.
- [ ] Current public features are described accurately, including regional live
      traffic, trajectories, atmospheric layers, attention explanation, replay
      telemetry, search/share state, and airport intelligence.
- [ ] The architecture and data-source boundaries distinguish live,
      deterministic, optional, and in-development capabilities.
- [ ] Local setup, verification, repository structure, and deployment guidance
      remain available without dominating the product overview.
- [ ] Current desktop and mobile production screenshots are stored in the
      repository and render from the README.
- [ ] Documentation and image paths pass diff-hygiene and link checks.
- [ ] Branch, final commit, pull request, required checks, and visual evidence
      are recorded before completion.

## Non-goals

- Changing application behavior, production data, infrastructure, or auth.
- Advertising the optional OpenAI drafting adapter as a public product feature.
- Replacing FT-403 neutral recruiter validation or FT-502 aviation-domain review.

## Verification evidence

Pending.
