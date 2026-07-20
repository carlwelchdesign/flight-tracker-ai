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
