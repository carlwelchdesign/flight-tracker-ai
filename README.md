# Flight Tracker AI

An advisory airline-operations intelligence console for fleet monitoring, aviation-weather correlation, and explainable dispatcher alerts.

## Start locally

Requirements for running the stack: Docker Desktop and Make. Running the full
local verification suite also requires Node.js 20.20.1 and npm 10.8.2.

```sh
cp .env.example .env
make dev
```

Open:

- Web interface: `http://localhost:3001`
- API health: `http://localhost:8080/health`
- API readiness: `http://localhost:8080/readiness`

The Rust API applies SQLx migrations at startup. The development stack uses PostgreSQL 17 with PostGIS 3.5 and keeps its data in a Docker volume.

Local requests use one explicit development administrator, but still pass through the signed assertion, database membership, role, revocation, and tenant checks used by hosted sessions. See [plans/IDENTITY_TENANT_ISOLATION.md](plans/IDENTITY_TENANT_ISOLATION.md) for the permission matrix and production setup.

Live NOAA ingestion is disabled by default. Before enabling it, create the configured operator in PostgreSQL, set `ENABLE_NOAA_WEATHER=true`, and keep the poll interval at 60 seconds or longer. See [plans/NOAA_INGESTION.md](plans/NOAA_INGESTION.md) for configuration, source-health semantics, and recovery guidance.

Stop the stack with `make down`. Run all local checks with `make verify`.

## Repository structure

```text
apps/api/       Rust/Axum operational API and workers
apps/web/       Next.js dispatcher interface
migrations/     Versioned PostgreSQL/PostGIS migrations
plans/          Product context, decisions, milestones, and tickets
compose.yaml    Reproducible local development stack
```

Read [plans/README.md](plans/README.md) before implementation. Every ticket uses a dedicated branch, ticket-scoped commits, and one pull request.

## Hosted identity and Vercel

The Next.js app can be deployed to Vercel with `AUTH_MODE=clerk`, Clerk publishable and secret keys, `INTERNAL_AUTH_KEY_ID`, `INTERNAL_AUTH_SECRET`, `AUTH_ASSERTION_ISSUER`, `AUTH_ASSERTION_AUDIENCE`, and an `API_BASE_URL` that points to the deployed Rust service. Configure the same active assertion key ID/secret, issuer, and audience on Rust, with `APP_ENV=production` and `AUTH_MODE=clerk`. Rust also accepts one explicit previous key pair during the controlled procedure in [plans/CREDENTIAL_ROTATION_RUNBOOK.md](plans/CREDENTIAL_ROTATION_RUNBOOK.md).

Vercel hosts the web interface and its server-side BFF. The Rust API and PostgreSQL/PostGIS remain separately deployed services unless the project later adopts Vercel Services. Before a hosted user can access operational data, create the Clerk organization and its matching app operator, identity, and membership records.
