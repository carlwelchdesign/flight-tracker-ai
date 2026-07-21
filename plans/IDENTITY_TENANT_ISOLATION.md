# Identity, Roles, and Tenant Isolation

FT-303 establishes a multi-tenant foundation while keeping the first pilot operationally simple. Hosted identity proves who the user is; the application database remains authoritative for which operator they may access and what they may do.

## Trust boundary

```text
browser session -> Next.js identity adapter -> short-lived internal assertion
                -> Rust signature/time validation -> database membership/revocation
                -> AuthContext -> tenant-scoped operation
```

- Clerk Organizations is the first hosted adapter because the web application targets Vercel and needs an active organization per operational tenant.
- The Rust API does not depend on Clerk types or metadata. Next.js converts a verified hosted session into a 30-second internal JWT containing provider, external subject, external tenant, session, and standard time/identity claims.
- The internal assertion is signed server-side with `INTERNAL_AUTH_SECRET`, is audience/issuer restricted, and is never exposed as an application credential or accepted from a query/body field.
- Rust validates signature, algorithm, issuer, audience, expiry, not-before time, and required claims. It then resolves the subject and tenant through app-owned records on every request.
- Missing, expired, disabled, revoked, cross-tenant, and insufficient-role requests fail closed.
- Health and readiness remain unauthenticated infrastructure probes. Operational APIs, streams, metrics, source evidence, and development replay controls require authorization.

Development uses the same signed assertion and database membership path. `AUTH_MODE=development` supplies one explicit local subject and tenant; it is forbidden in production. There is no unauthenticated development bypass.

## Persisted model

- `operators.identity_provider` and `operators.external_tenant_id` link one external organization to one app tenant.
- `auth_identities` stores a provider/subject identity and disable timestamp. It does not duplicate passwords, social profiles, or provider authorization metadata.
- `operator_memberships` links identities to operators with one app role and active/revoked status.
- `auth_session_revocations` immediately rejects a specific provider session until its expiry.
- `authorization_audit_events` records tenant, authenticated actor, action, target, timestamp, and structured metadata for membership/session administration.

## Role and permission matrix

| Capability | Viewer | Dispatcher | Operator | Administrator |
| --- | :---: | :---: | :---: | :---: |
| Read fleet, weather, alerts, source health/evidence | yes | yes | yes | yes |
| Subscribe to tenant event stream | yes | yes | yes | yes |
| Acknowledge, comment, dismiss, resolve alerts | no | yes | yes | yes |
| Use development replay controls | no | no | yes | yes |
| Read tenant-scoped service metrics | no | no | yes | yes |
| List/change memberships and revoke sessions | no | no | no | yes |

Roles are ordered only for this fixed policy version; handlers ask for named permissions, not numeric role levels. Hosted-provider organization roles are not authoritative for application actions.

## Tenant selection

The active hosted organization becomes the assertion's external tenant. Rust maps it to exactly one `operator_id`. The browser cannot choose an operator using a query parameter, JSON body, or forwarded header. A user who belongs to multiple operators changes the active organization through the identity provider, producing a new verified assertion.

## Endpoint policy

- Public: `GET /health`, `GET /readiness`.
- Operational read: fleet list/detail/timeline, SSE, weather, source evidence, source health, alerts, auth context.
- Dispatcher write: alert actions. Actor and operator are taken from `AuthContext`.
- Operator write: development replay controls.
- Operator diagnostic: `/metrics`, filtered to the active operator where labels contain tenant data.
- Administrator: membership list/update and session revocation.

Background ingestion and replay workers continue using configured operator identities; they do not impersonate a human.

## Safe session behavior

- The Next.js page performs a secure auth-context read before returning operational data.
- Client code periodically rechecks the context. A 401 (missing/expired hosted session) or 403 (revoked membership/session) replaces the operational console with a non-data-bearing signed-out/revoked state and a sign-in action.
- The BFF never reuses assertions beyond 30 seconds. Revoked hosted sessions stop producing assertions; app revocation is checked independently on every Rust request.
- SSE reconnects must reauthenticate and are filtered to the authenticated operator.

## Provisioning and rollback

Development bootstrap may create the configured local operator, identity, and administrator membership only when `APP_ENV=development` and `AUTH_MODE=development`.

Production provisioning requires a Clerk organization plus an app operator/identity/membership mapping. Removing or revoking a membership immediately denies access without deleting historical audit records. Rolling back the web identity adapter does not require rewriting authorization data or tenant-scoped repositories.

For Vercel, configure `AUTH_MODE=clerk`, Clerk's publishable and secret keys, the internal assertion secret/issuer/audience, and `API_BASE_URL` on the Next.js project. The Rust deployment receives the same assertion settings plus `APP_ENV=production`, `AUTH_MODE=clerk`, and `DATABASE_URL`. The shared secret is a server-to-server credential and must never use the checked-in development value in a hosted environment.

## Verification requirements

- Pure permission and claim-validation tests for every role and failure class.
- PostGIS tests proving cross-tenant list/detail/source/alert/action access fails closed.
- SSE tests proving replay and live delivery do not cross tenants.
- Concurrency/idempotency tests for membership changes and session revocation.
- UI tests for signed-in, expired, revoked, insufficient-role, loading, and recovery states.
- Production configuration rejects development auth and missing secrets.
