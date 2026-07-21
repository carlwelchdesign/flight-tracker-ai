# Hosted Portfolio Deployment Runbook

## Approved shape

- **Web:** Vercel project `flight-tracker-ai`, Git-connected to this repository
  with `apps/web` as its root and Node.js 20.x.
- **API and supervised workers:** Render Docker web service from
  [`render.yaml`](../render.yaml). The free service is acceptable for this
  hobby portfolio, not an availability claim: it sleeps after 15 idle minutes
  and can take about one minute to wake.
- **Database:** Neon Free Postgres in the same US West region when available,
  with PostGIS enabled, a direct or pooled TLS URL, instant restore, and one
  protected snapshot. Neon is separate from Vercel and Render lifecycle.
- **Identity:** Clerk Organizations. The Vercel server signs short-lived
  internal assertions; Render verifies the same named secret. No assertion
  secret is exposed to browser code.

Vercel Services remains unsuitable for this release because Rust services are
not yet a validated runtime there and the supervised ingestion/replay processes
need a persistent container lifecycle. Render Free may stop those workers while
idle; deterministic replay restarts cleanly when the API wakes.

## Cost and reliability boundary

This configuration targets a recruiter portfolio at zero base cost. Render
Free has no SLA, sleeps on idle, and can restart. Neon Free currently provides
0.5 GB storage, 100 CU-hours per month, connection pooling, PostGIS, instant
restore, and one snapshot. Revalidate these provider terms immediately before
provisioning because free-plan limits can change.

The UI names the possible cold start and offers a retry. Never describe this as
production airline infrastructure, high availability, or a commercial SLA.

## Provisioning order

1. Create the Neon project in a US West region. Enable PostGIS and verify:

   ```sql
   CREATE EXTENSION IF NOT EXISTS postgis;
   SELECT extversion FROM pg_extension WHERE extname = 'postgis';
   ```

2. Create the Render Blueprint from this repository. Supply the Neon TLS
   `DATABASE_URL` and a new random `INTERNAL_AUTH_SECRET` when Render prompts.
   Keep the generated value outside Git, PRs, screenshots, and chat.
3. Install Clerk on the Vercel project, create one organization and reviewer
   user, then configure Vercel preview and production variables listed below.
4. Use the same internal secret and key ID on Vercel and Render. Deploy Render
   first, verify `/health` and `/readiness`, then set Vercel `API_BASE_URL` to
   the HTTPS Render origin.
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

Render deploys `main` only after GitHub checks pass. Its health check is
`/health`; the API also runs SQLx migrations before accepting traffic. Vercel
creates branch previews through Git integration. The release order is database
backup/snapshot, Render deploy and health verification, Vercel preview, browser
smoke, then promotion of the tested Vercel artifact.

Rollback the web by repointing the prior Vercel deployment. Roll back Render to
its prior successful image only when the database migration is backward
compatible. Otherwise stop promotion, restore the Neon snapshot/restore point
into an isolated branch, verify PostGIS and migration state, and follow
[`BACKUP_RESTORE_RUNBOOK.md`](BACKUP_RESTORE_RUNBOOK.md).

## Required hosted evidence

- [ ] Vercel Git connection creates a distinct pull-request preview.
- [ ] Render deployment ID, commit, health, readiness, and worker status pass.
- [ ] Neon region, PostGIS version, pooling path, snapshot, and isolated restore
      are recorded without exposing its connection string.
- [ ] Vercel and Render use matching active key IDs and distinct preview versus
      production secret references.
- [ ] Clerk sign-in, organization selection, membership, session expiry, and
      revoked-session behavior pass.
- [ ] Browser smoke covers replay, flight evidence, alert action, cold-start or
      degraded state, source labels, and optional positions disabled.
- [ ] Hosted FT-401 verifier passes with sanitized output.
- [ ] Response headers, TLS, bounded logs, and basic availability monitoring
      pass.
- [ ] FT-403 independent neutral reviewer passes the unfacilitated protocol.
- [ ] The candidate contains no certification, operational-authority,
      commercial-SLA, or real-operator claim.
