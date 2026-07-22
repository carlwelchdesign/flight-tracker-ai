# FT-418 — Upgrade the web runtime to Node.js 24 LTS

Status: Complete

Branch: `chore/ft-418-node-24-runtime`
Implementation commit: `6a9adc9`
Pull request: [#58](https://github.com/carlwelchdesign/flight-tracker-ai/pull/58)
Owner: Frontend and platform engineering

Move the Next.js build and runtime from end-of-life Node.js 20 to Node.js 24
LTS before Vercel disables new Node 20 deployments on 2026-10-01.

Dependencies: FT-404

## Acceptance checklist

- [x] `apps/web/package.json` selects Node.js `24.x`, overriding the deprecated
      Vercel project setting for the next deployment.
- [x] Local development, GitHub Actions, and the web Docker image use the same
      current Node.js 24 LTS patch line.
- [x] Node type definitions and lockfile root metadata match the Node.js 24
      runtime without unnecessary dependency upgrades.
- [x] Current setup and deployment documentation names Node.js 24 instead of
      Node.js 20; historical ticket evidence remains unchanged.
- [x] A clean Node.js 24 install passes dependency audit, lint, TypeScript, all
      web tests, and the production build without `EBADENGINE` warnings.
- [x] Vercel builds the ticket commit with Node.js 24 and no Node 20
      deprecation or Mapbox engine warning.
- [x] The hosted tracker and public social image remain available after the
      runtime-only change.
- [x] Branch, commits, PR, CI, deployment, and verification evidence are
      recorded before completion.

## Non-goals

- Changing application behavior, UI, data providers, or public contracts.
- Upgrading Next.js, React, MapLibre, or unrelated dependencies.
- Moving the Rust service onto Vercel.

## Verification evidence

- Vercel production deployment logs for pre-upgrade commit `5168424` showed the
  Node 20 deprecation as an `Error` and an `EBADENGINE` warning from
  `@mapbox/jsonlint-lines-primitives`, but the same log continued through
  `Build Completed`, `Deployment completed`, and `Ready`. This was a dated
  runtime blocker, not a failed current deployment.
- Official Vercel guidance disables new Node 20 deployments on 2026-10-01 and
  directs projects to select `engines.node: 24.x`. Node.js 24.18.0 is the
  current v24 LTS patch used by `.nvmrc`, GitHub Actions, and both Docker stages;
  Vercel owns the patch version within its `24.x` runtime contract.
- A clean install under Node.js 24.18.0 and npm 11.16.0 installed 483 packages
  with zero vulnerabilities and no engine mismatch. Only `@types/node` and its
  `undici-types` dependency changed in the locked tree. Lint, TypeScript, all
  142 tests across 43 files, and the production Next.js build pass.
- The official Docker Hub registry reports
  `node:24.18.0-bookworm-slim` active. Docker Desktop reached the registry
  metadata step but its local pull stalled and was canceled; the same exact
  Node runtime passed the clean local install and application verification.
- GitHub Actions run
  [29939055952](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29939055952)
  passed Rust, web, and API/PostGIS checks. The web job ran the clean install,
  audit, lint, TypeScript, 142 tests, production build, and browser-secret scan
  on Node.js 24.18.0.
- Vercel project `flight-tracker-ai` was updated from Node.js `20.x` to `24.x`.
  A cache-free redeploy of commit `6a9adc9`, deployment
  `dpl_H1YnJnYCsfijHyEyqGS1KwFoksBS`, completed `Ready` without the Node 20
  deprecation, `EBADENGINE`, unsupported-engine, or stale-project-setting
  messages.
- Authenticated preview probes returned HTTP 200 for the root tracker and its
  Open Graph image. The HTML retained the branded Open Graph title, and the
  image remained a valid 1200 by 630 PNG.
