# Hosted Portfolio Deployment Runbook

## Approved shape

- **Web:** Vercel project `flight-tracker-ai`, Git-connected to this repository
  with `apps/web` as its root and Node.js 20.x.
- **API and supervised workers:** Explicit staging and production Render Docker
  web services from [`render.yaml`](../render.yaml). Free services are
  acceptable for this hobby portfolio, not an availability claim: they sleep
  after 15 idle minutes and can take about one minute to wake.
- **Database:** Separate Neon Free Postgres projects for production
  (`neon-bronze-curtain`) and staging (`neon-bistre-lantern`) in AWS
  `us-east-1`, aligned with Render Virginia, with PostGIS enabled and direct
  and pooled TLS URLs. Neon is separate from Vercel and Render lifecycle.
- **Identity:** Clerk Organizations. The Vercel server signs short-lived
  internal assertions; Render verifies the same named secret. No assertion
  secret is exposed to browser code.

Vercel Services remains unsuitable for this release because Rust services are
not yet a validated runtime there and the supervised ingestion/replay processes
need a persistent container lifecycle. Render Free may stop those workers while
idle; deterministic replay restarts cleanly when the API wakes.

## Cost and reliability boundary

This configuration targets a recruiter portfolio at zero base cost. Render
Free has no SLA, sleeps on idle, can restart, and shares 750 running instance
hours per workspace per calendar month. The staging service should remain idle
outside release verification. Render's managed preview environments require a
Pro workspace, so this Blueprint uses an explicit free staging service instead.
Neon Free currently provides
0.5 GB storage, 100 CU-hours per month, connection pooling, PostGIS, instant
restore, and one snapshot. Revalidate these provider terms immediately before
provisioning because free-plan limits can change.

The UI names the possible cold start and offers a retry. Never describe this as
production airline infrastructure, high availability, or a commercial SLA.

## Provisioning order

1. Provision Neon through the Vercel Marketplace in AWS `us-east-1`. Keep
   production separate from staging with distinct projects or branches, enable
   PostGIS on both, and verify:

   ```sql
   CREATE EXTENSION IF NOT EXISTS postgis;
   SELECT extversion FROM pg_extension WHERE extname = 'postgis';
   ```

2. Create the Render Blueprint from this repository. Supply distinct Neon TLS
   `DATABASE_URL` and random `INTERNAL_AUTH_SECRET` values for
   `flight-tracker-api-staging` and `flight-tracker-api` when Render prompts.
   Keep all values outside Git, PRs, screenshots, and chat. Render's automatic
   preview environments are intentionally disabled because they require Pro
   and omit `sync: false` values.
3. Install Clerk on the Vercel project, create one organization and reviewer
   user, then configure Vercel preview and production variables listed below.
4. Match each Vercel environment to its Render counterpart: Preview uses the
   staging API URL and secret; Production uses the production API URL and
   secret. Both use the same named key ID. Verify `/health` and `/readiness`
   before setting each Vercel `API_BASE_URL`.
5. After migrations finish, bootstrap the exact Clerk organization/user pair:

   ```sh
   psql "$DATABASE_URL" \
     -v clerk_org_id="$CLERK_ORG_ID" \
     -v clerk_user_id="$CLERK_USER_ID" \
     -f scripts/bootstrap_hosted_portfolio.sql
   ```

   The CI check `scripts/verify_hosted_portfolio_bootstrap.sh` applies the
   script twice and proves it produces exactly one tenant-bound active
   administrator membership.

6. Deploy a private Vercel preview, run every verification below, and collect
   the FT-403 neutral-review observation. Promote that exact artifact only
   after both ticket gates pass.

## Provisioning evidence

On 2026-07-21, the linked Vercel project provisioned Neon resources
`neon-bronze-curtain` for production and `neon-bistre-lantern` for staging, and
connected Clerk resource `clerk-celeste-door`. The staging database is attached
only to Preview under a namespaced variable set, while the production database
retains its original connection. Vercel reports both databases as available.
It injected the expected pooled and unpooled Neon connection variables plus
`CLERK_SECRET_KEY` and `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY`.
`AUTH_MODE`, `OPERATIONS_MODE`, `INTERNAL_AUTH_KEY_ID`,
`AUTH_ASSERTION_ISSUER`, and `AUTH_ASSERTION_AUDIENCE` are configured for all
three Vercel environments. Preview and Production now have distinct
`API_BASE_URL` and `INTERNAL_AUTH_SECRET` values matching their Render service.
Clerk Production now uses live keys while Preview uses test keys. The production
domain is `flight-tracker-ai-one.vercel.app`, required organization membership
is enabled, and organization `Flight Tracker Portfolio` exists. Production
deployment `dpl_FXv3uAUVCKCRTTfTm5xRj7rn1pWE` serves the public signed-out state
and production Clerk sign-in flow without exposing configuration details.
The pooled and direct database URLs both require TLS, target AWS `us-east-1`,
and differ as expected by pooler usage. The direct connection enabled and
reported PostGIS `3.5.0`. On 2026-07-21, Neon retained the manual production
snapshot `main at 2026-07-21 20:46:51 UTC (manual)` with no expiry. An isolated
point-in-time branch restored production `main` from 13:45 PDT and matched the
source with PostGIS `3.5.0`, 14 successful SQLx migrations, one operator, and
zero identities, memberships, alerts, or alert actions. The temporary restore
branch was deleted after verification; production `main` was not modified. An
earlier 13:05 PDT candidate correctly failed the migration gate because it
predated the application schema and was also deleted. No secret value or
connection string is recorded here.

Render Blueprint `exs-d9ft018okrbs738q5r60` created production service
`srv-d9ft2gn7f7vs739ass40` at `https://flight-tracker-api-cpxg.onrender.com`
and staging service `srv-d9ft2gn7f7vs739ass3g` at
`https://flight-tracker-api-staging.onrender.com`. The first staging start
failed closed because a sensitive Vercel pull represented its database URL as
the literal placeholder `[SENSITIVE]`; the authenticated provider value was
then applied directly without exposing it. Production health and migrations
passed. Hosted readiness exposed an idle `alert_projection` heartbeat defect,
which commit `a665494` fixes with a periodic supervised heartbeat and an HSTS
response layer; commit `6a4929e` passes CI run
[`29865574640`](https://github.com/carlwelchdesign/flight-tracker-ai/actions/runs/29865574640).
Render production deploy `dep-d9ftbi61a83c7396hbsg` and staging deploy
`dep-d9ftf2naqgkc738okfig` run commit `6a4929e`. Both return exact HTTP 200
health/readiness contracts with HSTS, PostGIS, migrations, and all four
critical workers ready. A short-lived deployment-verifier identity proved each
environment's distinct internal assertion secret and was then removed from
both databases. Public verifier evidence passes for protected Vercel preview
`dpl_FNbngNWmbKNSafY5rvNypHjPaPzS` and publication-ready production deployment
`dpl_FXv3uAUVCKCRTTfTm5xRj7rn1pWE` without emitting origins, bodies, or secrets.

## Vercel server-only configuration

| Variable | Preview | Production | Secret |
| --- | --- | --- | --- |
| `API_BASE_URL` | preview/staging API origin | production API origin | No |
| `AUTH_MODE` | `clerk` | `clerk` | No |
| `OPERATIONS_MODE` | `evaluation` | `evaluation` | No |
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Clerk preview key | Clerk production key | Public by design |
| `CLERK_SECRET_KEY` | Clerk preview secret | Clerk production secret | Yes |
| `INTERNAL_AUTH_KEY_ID` | named active key | named active key | No |
| `INTERNAL_AUTH_SECRET` | environment-specific value | production value | Yes |
| `AUTH_ASSERTION_ISSUER` | `flight-tracker-web` | same | No |
| `AUTH_ASSERTION_AUDIENCE` | `flight-tracker-api` | same | No |

Preview must not share a production database or internal assertion secret. No
secret uses a `NEXT_PUBLIC_` prefix.

## Controlled release and rollback

During PR verification both Render services deploy the FT-404 feature branch;
the final promotion commit changes both service branches to `main`. Staging
deploys only after GitHub checks pass. Production requires manual promotion
after staging and browser smoke. Render Free does not support a configurable
maximum shutdown delay, so the Blueprint intentionally uses the platform
default.
Both use `/health`, and the API runs SQLx migrations before accepting traffic.
Vercel creates branch previews through Git integration. The release order is
database backup/snapshot, staging Render deploy and health verification,
Vercel preview, browser smoke, production Render promotion, then promotion of
the tested Vercel artifact.

Rollback the web by repointing the prior Vercel deployment. Roll back Render to
its prior successful image only when the database migration is backward
compatible. Otherwise stop promotion, restore the Neon snapshot/restore point
into an isolated branch, verify PostGIS and migration state, and follow
[`BACKUP_RESTORE_RUNBOOK.md`](BACKUP_RESTORE_RUNBOOK.md).

## Required hosted evidence

- [x] Vercel Git connection creates a distinct pull-request preview.
- [x] Staging and production Render deployment IDs, commits, health, readiness,
      and worker status pass.
- [x] Neon region, PostGIS version, pooling path, snapshot, and isolated restore
      are recorded without exposing its connection string.
- [x] Vercel and Render use matching active key IDs and distinct preview versus
      production secret references.
- [ ] Clerk sign-in, organization selection, membership, session expiry, and
      revoked-session behavior pass.
- [ ] Browser smoke covers replay, flight evidence, alert action, cold-start or
      degraded state, source labels, and optional positions disabled.
- [ ] Hosted FT-401 verifier passes with sanitized output.
- [x] Response headers, TLS, bounded logs, and basic availability monitoring
      pass.
- [ ] FT-403 independent neutral reviewer passes the unfacilitated protocol.
- [ ] The candidate contains no certification, operational-authority,
      commercial-SLA, or real-operator claim.

Before authenticated browser checks, run the sanitized public-boundary verifier:

```sh
python3 scripts/verify_ft404_public_surface.py \
  --environment-reference preview-candidate \
  --web-origin "$WEB_ORIGIN" \
  --api-origin "$API_ORIGIN" \
  --allow-deployment-protection
```

A protected preview may pass with `publication_ready: false`. Before promotion,
run it again against the candidate without `--allow-deployment-protection`; the
root must use the Clerk sign-in boundary, all browser security headers must
match, public health/readiness must be exact, and the detailed API must return
the exact unauthenticated denial. The emitted evidence does not include origins,
redirect URLs, headers, bodies, credentials, cookies, or tokens.
