# FT-417 — Add branded social-share metadata

Status: Complete

Branch: `feat/ft-417-social-share-metadata`
Implementation commit: `8715401`
Pull request: [#56](https://github.com/carlwelchdesign/flight-tracker-ai/pull/56)
Owner: Frontend product engineering

Make the public tracker produce an accurate, branded link preview instead of
the hosting platform's generic preview when its production URL is shared.

Dependencies: FT-405

## Acceptance checklist

- [x] The root page publishes a stable title, useful description, application
      name, canonical production URL, and index/follow policy.
- [x] Open Graph metadata identifies the public tracker as a website and uses a
      1200 by 630 branded preview image with descriptive alternative text.
- [x] Twitter/X metadata requests a large-image card and uses the same truthful
      title, description, and branded image.
- [x] The preview copy describes live regional aircraft, deterministic replay,
      aviation weather, trajectories, and explainable attention without
      implying certified or operational use.
- [x] Metadata configuration is isolated from the layout and covered by a
      focused contract test.
- [x] Lint, TypeScript, focused tests, the full web suite, and the production
      build pass.
- [x] Rendered HTML and the generated image route are verified locally and on
      the hosted production URL.
- [x] Branch, commit, PR, required checks, and production evidence are recorded
      before completion.

## Non-goals

- Per-aircraft dynamic share images.
- Persisting share requests or introducing analytics.
- Changing public tracker behavior, data sources, or operational boundaries.

## Verification evidence

- `site-metadata.ts` owns the stable product contract, while the root layout
  only exports it. Its focused test verifies the canonical URL, application
  name, Open Graph dimensions and alternative text, Twitter/X large-card
  contract, robots policy, implemented capabilities, and non-operational
  portfolio wording.
- `opengraph-image.tsx` generates one static image without external assets,
  user data, provider records, or ephemeral aircraft state. The hosted identity
  proxy explicitly permits the image route, and its proxy test proves an
  anonymous crawler is not sent through Clerk.
- Local verification passes all 142 web tests across 43 files, lint with zero
  warnings, TypeScript, the clean Next.js production build, and diff hygiene.
  The build prerenders `/opengraph-image`; local HTTP verification returned the
  complete metadata and an anonymous HTTP 200 `image/png` response at 1200 by
  630 pixels.
- GitHub Actions run
  [29937128296](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29937128296)
  passed Rust, web, and API/PostGIS checks. The Vercel integration also passed
  for PR #56.
- Production deployment `dpl_2VhnnX6Q1CN7XhcrMvQf73fh8Nd1` was promoted and
  assigned to `flight-tracker-ai-one.vercel.app`. Anonymous production requests
  returned HTTP 200 for both the root HTML and generated PNG. The HTML contains
  the canonical production URL, index/follow policy, Open Graph website/title/
  description/image/type/dimensions/alt fields, and Twitter/X large-card fields;
  the image is a verified 1200 by 630 PNG with no authentication redirect.
