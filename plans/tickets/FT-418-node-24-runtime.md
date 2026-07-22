# FT-418 — Upgrade the web runtime to Node.js 24 LTS

Status: In progress

Branch: `chore/ft-418-node-24-runtime`
Final commit: Pending
Pull request: Pending
Owner: Frontend and platform engineering

Move the Next.js build and runtime from end-of-life Node.js 20 to Node.js 24
LTS before Vercel disables new Node 20 deployments on 2026-10-01.

Dependencies: FT-404

## Acceptance checklist

- [ ] `apps/web/package.json` selects Node.js `24.x`, overriding the deprecated
      Vercel project setting for the next deployment.
- [ ] Local development, GitHub Actions, and the web Docker image use the same
      current Node.js 24 LTS patch line.
- [ ] Node type definitions and lockfile root metadata match the Node.js 24
      runtime without unnecessary dependency upgrades.
- [ ] Current setup and deployment documentation names Node.js 24 instead of
      Node.js 20; historical ticket evidence remains unchanged.
- [ ] A clean Node.js 24 install passes dependency audit, lint, TypeScript, all
      web tests, and the production build without `EBADENGINE` warnings.
- [ ] Vercel builds the ticket commit with Node.js 24 and no Node 20
      deprecation or Mapbox engine warning.
- [ ] The hosted tracker and public social image remain available after the
      runtime-only change.
- [ ] Branch, commits, PR, CI, deployment, and verification evidence are
      recorded before completion.

## Non-goals

- Changing application behavior, UI, data providers, or public contracts.
- Upgrading Next.js, React, MapLibre, or unrelated dependencies.
- Moving the Rust service onto Vercel.

## Verification evidence

Pending.
