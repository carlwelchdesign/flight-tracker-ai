# FT-405 hosted live-source failure verification

Status: In progress

The closeout branch has an isolated Vercel Preview override for
`API_BASE_URL` pointing to a reserved, non-resolving `.invalid` origin. This
forces the deployed Next.js public proxy to fail without changing production,
the shared Preview environment, the Rust service, or any provider data.

The branch override must be removed after browser verification. Final evidence
will record the exact preview deployment, visible fallback state, retry
behavior, responsive layout, and cleanup result without recording environment
variable values or credentials.
