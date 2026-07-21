# Credential Rotation and Emergency Revocation Runbook

FT-401 control procedure, last reviewed 2026-07-21. This runbook defines the application-side rotation contract and the evidence required from hosted environments. It does not claim that Vercel, the Rust host, Clerk, PostgreSQL, or a commercial provider has been configured or drilled.

## Secret inventory and ownership

| Credential | Runtime boundary | Rotation owner | Application control | Hosted evidence still required |
| --- | --- | --- | --- | --- |
| Internal assertion HMAC key | Next.js signer and Rust verifier | Security/Platform | Named active key, optional named previous key, 30-second signer lifetime, 60-second verifier maximum | Environment-separated secret-store references and a completed normal/emergency drill |
| Clerk secret key | Next.js server and Clerk | Security/Identity | Server-only Clerk adapter; never forwarded to Rust/browser | Clerk rotation/revocation procedure and hosted sign-in smoke |
| PostgreSQL credential | Rust runtime and migration job | Platform/Database | Server-side connection only | Managed-database rotation, pool reconnection, migration, and rollback evidence |
| NOAA/provider credential | Rust ingestion adapter | Backend/Platform | Provider-specific server-side adapter boundary | Provider console/API revocation, replacement, rate/health, and attribution smoke |

Never copy a credential, token, raw secret-store value, or full assertion into Git, a PR, a ticket, chat, screenshots, logs, or drill evidence. Evidence uses key IDs, secret version references, timestamps, deployment IDs, correlation IDs, and pass/fail results only.

## Internal assertion key contract

- Next.js signs every assertion with `INTERNAL_AUTH_KEY_ID` and `INTERNAL_AUTH_SECRET`; the JWT header includes the key ID as `kid`.
- Rust always verifies the named active key. During a planned cutover it may also verify exactly one previous pair from `INTERNAL_AUTH_PREVIOUS_KEY_ID` and `INTERNAL_AUTH_PREVIOUS_SECRET`.
- The previous variables are an all-or-nothing pair. Key IDs must be distinct safe identifiers; active and previous secrets must differ and contain at least 32 bytes.
- When two keys are configured, assertions without a `kid` fail closed. A single-key verifier temporarily accepts legacy no-`kid` assertions only to allow the initial API-first rollout of this protocol.
- The web lifetime is 30 seconds; Rust rejects assertions whose issued-to-expiry lifetime exceeds 60 seconds. Removing a previous key stops accepting it immediately at the verifier, regardless of token expiry.

## Planned zero-downtime rotation

1. Open a controlled change record with owners, affected environments, old/new key IDs, deployment order, observation window, rollback decision point, and evidence destination. Do not record key material.
2. Generate the new secret inside the approved managed secret system. Use a unique non-semantic key ID such as a secret version or rotation date plus sequence.
3. On the Rust environment, set the new ID/secret as active and the current ID/secret as previous. Deploy Rust first.
4. Verify readiness, an authenticated `/api/system/health` request, a normal BFF request, and rejection of an unknown `kid`. Confirm no credential value appears in logs or responses.
5. On the matching Next.js environment, set the new ID/secret as active and deploy. Verify sign-in, organization selection, BFF reads/actions, SSE reconnect, and audit actor/tenant evidence.
6. Observe for at least 65 seconds (the verifier maximum lifetime plus configured leeway) and confirm the old key ID no longer appears in accepted-request telemetry. Never log the assertion or secret to obtain this signal.
7. Remove both previous-key variables from Rust and deploy again. Prove a test assertion signed by the retired key is rejected while the active key succeeds.
8. Revoke/delete the retired secret version in the managed secret system according to its recovery policy. Close the change only after the evidence template is complete.

## Emergency revocation

When a key may be compromised, containment takes priority over availability:

1. Declare the incident and record the suspected key ID, environments, detection time, and incident reference without copying the secret.
2. Generate a replacement in the managed secret system and update the Next.js signer and Rust active verifier key through the fastest controlled deployment path.
3. Do **not** configure the suspected key as previous. Remove it from every verifier immediately; short user-facing interruption is acceptable while the signer and verifier converge.
4. Revoke affected Clerk sessions or identities independently when their credentials or sessions may also be exposed. Rotating the internal HMAC key does not revoke hosted sessions by itself.
5. Verify the suspected key fails, the replacement succeeds, public probes remain minimal, authenticated diagnostics work, and no cross-tenant or privileged action occurred during the exposure window.
6. Search approved logs/build artifacts for evidence of exposure without printing matched secret content. Follow the incident sequence in [`DATA_LIFECYCLE_INCIDENT_POLICY.md`](DATA_LIFECYCLE_INCIDENT_POLICY.md).

## Rollback

- Before compromise is suspected, rollback the web signer to the previous key only while Rust still lists it as previous.
- Never restore a suspected compromised key for availability.
- If the new Rust deployment fails before the web cutover, restore the prior single active key configuration and investigate.
- If the web cutover fails after Rust accepts both keys, keep Rust in overlap and restore the prior web signer; then correct and repeat.

## Drill evidence template

| Evidence | Required value |
| --- | --- |
| Change/incident reference | Controlled opaque reference |
| Environment and deployment IDs | Preview/staging/production identifiers |
| Active and retired key IDs | IDs only; never secrets |
| Rust overlap deployed | Timestamp and deployment ID |
| Web cutover deployed | Timestamp and deployment ID |
| Active key success | Authenticated health, BFF, SSE, and action result references |
| Unknown/retired key rejection | Timestamp, expected 401, safe correlation ID |
| Browser/build/log leak check | Pass/fail and artifact/log range |
| Previous key removed/revoked | Timestamp and managed-secret version reference |
| Owners/approvers | Security and Platform names/roles |
| Follow-ups | Owned tickets with deadlines |

F401-001 remains open until hosted environment separation is configured and both a planned rotation and emergency revocation drill provide this evidence.
