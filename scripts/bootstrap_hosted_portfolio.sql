\set ON_ERROR_STOP on

\if :{?clerk_org_id}
\else
  \error 'clerk_org_id is required'
\endif
\if :{?clerk_user_id}
\else
  \error 'clerk_user_id is required'
\endif

BEGIN;

INSERT INTO operators (
    id,
    code,
    display_name,
    identity_provider,
    external_tenant_id
) VALUES (
    '9c704a09-a62c-43d5-bac6-94ea2fd53b32',
    'SIM',
    'Portfolio Simulation',
    'clerk',
    :'clerk_org_id'
)
ON CONFLICT (id) DO UPDATE SET
    code = EXCLUDED.code,
    display_name = EXCLUDED.display_name,
    identity_provider = EXCLUDED.identity_provider,
    external_tenant_id = EXCLUDED.external_tenant_id;

WITH hosted_identity AS (
    INSERT INTO auth_identities (id, provider, subject, display_name)
    VALUES (gen_random_uuid(), 'clerk', :'clerk_user_id', 'Portfolio reviewer')
    ON CONFLICT (provider, subject) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        disabled_at = NULL
    RETURNING id
)
INSERT INTO operator_memberships (
    id,
    operator_id,
    identity_id,
    role,
    status
)
SELECT
    gen_random_uuid(),
    '9c704a09-a62c-43d5-bac6-94ea2fd53b32',
    hosted_identity.id,
    'administrator',
    'active'
FROM hosted_identity
ON CONFLICT (operator_id, identity_id) DO UPDATE SET
    role = EXCLUDED.role,
    status = 'active',
    revoked_at = NULL,
    updated_at = NOW();

COMMIT;
