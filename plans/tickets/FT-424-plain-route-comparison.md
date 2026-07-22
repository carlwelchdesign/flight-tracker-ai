# FT-424 — Rewrite the public AI panel as a route comparison

Status: Complete

Branch: `fix/ft-424-plain-route-comparison`
Final implementation commit: `a2c06f6`
Pull request: [#68](https://github.com/carlwelchdesign/flight-tracker-ai/pull/68)
Owner: Frontend engineering and product design

Replace the internal, AI-centric presentation of the fixed public drafting demo
with a plain-language explanation of the route tradeoff it represents. Keep the
existing fixed-input OpenAI integration and review boundary, but make them
supporting provenance rather than the product headline.

Dependencies: FT-504

## Acceptance checklist

- [x] The panel leads with the route comparison and explains the fixed example
      in ordinary language.
- [x] Fixture identifiers such as `north_clear` are formatted for people in all
      visible summary copy.
- [x] The main result emphasizes hazard clearance and added distance without
      labels such as “Human-reviewed AI,” “deterministic source facts,” or
      “generated draft.”
- [x] Model, evidence-version, rules, and citations remain available in one
      collapsed “How this was calculated” disclosure.
- [x] One concise demonstration boundary replaces repeated synthetic,
      operational, approval, and review disclaimers.
- [x] Loading, failure, cached regeneration, no-auto-action, and fixed-input
      behavior remain unchanged.
- [x] Focused tests, the full web suite, lint, typecheck, production build,
      browser verification, and diff hygiene pass.
- [x] Branch, final commit, pull request, required checks, and hosted evidence
      are recorded before completion.

## Non-goals

- Changing the Rust/OpenAI request, prompt, validation, cache, or API contract.
- Adding a route selector, free-form prompt, approval, send, or operational
  action.
- Claiming that the sample route is live, filed, cleared, or suitable for flight
  planning.

## Verification evidence

- `npm test -- --run src/components/operations/public-ai-draft-panel.test.tsx`
  — 2 focused tests passed.
- `npm test` — 47 files and 154 tests passed.
- `npm run lint`, `npm run typecheck`, and `npm run build` passed on Node.js
  24.18.0; `git diff --check` passed.
- Local browser verification at `http://localhost:3002/` exercised the real
  Next.js-to-Rust boundary, opened the calculation disclosure, confirmed the
  humanized `North clear` result, and reported no browser errors.
- The main panel contains no model name, raw fixture identifier, approval
  state, generated-draft label, or deterministic-fixture language. Provider,
  dataset, rule, and source evidence remain in the collapsed disclosure.
- Implementation commit: `a2c06f6`; delivery PR:
  [#68](https://github.com/carlwelchdesign/flight-tracker-ai/pull/68).
- GitHub checks passed: API and PostGIS smoke test, Rust checks, Web checks,
  Vercel, and Vercel Preview Comments. PR #68 merged as `a14bfdb`.
- Production deployment `dpl_2WP5itNQ5Faejb6BBkt4gHfcw2QV` is ready and
  assigned to [flight-tracker-ai-one.vercel.app](https://flight-tracker-ai-one.vercel.app/).
  Hosted browser verification loaded the OpenAI-backed `North clear` sample,
  expanded the calculation details, confirmed the plain-language tradeoff and
  humanized evidence, and reported no browser errors.
