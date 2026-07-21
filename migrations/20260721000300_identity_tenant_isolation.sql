ALTER TABLE operators
    ADD COLUMN identity_provider TEXT,
    ADD COLUMN external_tenant_id TEXT,
    ADD CONSTRAINT operators_external_tenant_pair CHECK (
        (identity_provider IS NULL AND external_tenant_id IS NULL)
        OR (
            identity_provider IS NOT NULL AND btrim(identity_provider) <> ''
            AND external_tenant_id IS NOT NULL AND btrim(external_tenant_id) <> ''
        )
    ),
    ADD CONSTRAINT operators_external_tenant_unique
        UNIQUE (identity_provider, external_tenant_id);

CREATE TABLE auth_identities (
    id UUID PRIMARY KEY,
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    subject TEXT NOT NULL CHECK (btrim(subject) <> ''),
    display_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disabled_at TIMESTAMPTZ,
    UNIQUE (provider, subject),
    CHECK (display_name IS NULL OR btrim(display_name) <> '')
);

CREATE TABLE operator_memberships (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    identity_id UUID NOT NULL REFERENCES auth_identities(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'dispatcher', 'operator', 'administrator')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE (operator_id, identity_id),
    UNIQUE (operator_id, id),
    CHECK (
        (status = 'active' AND revoked_at IS NULL)
        OR (status = 'revoked' AND revoked_at IS NOT NULL)
    )
);

CREATE INDEX operator_memberships_identity_status_idx
    ON operator_memberships (identity_id, status);

CREATE TABLE auth_session_revocations (
    id UUID PRIMARY KEY,
    provider TEXT NOT NULL CHECK (btrim(provider) <> ''),
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    identity_id UUID NOT NULL REFERENCES auth_identities(id),
    operator_id UUID NOT NULL REFERENCES operators(id),
    revoked_by_identity_id UUID NOT NULL REFERENCES auth_identities(id),
    reason TEXT NOT NULL CHECK (btrim(reason) <> ''),
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    UNIQUE (provider, session_id),
    FOREIGN KEY (operator_id, identity_id)
        REFERENCES operator_memberships(operator_id, identity_id),
    CHECK (expires_at > revoked_at)
);

CREATE INDEX auth_session_revocations_expiry_idx
    ON auth_session_revocations (expires_at);

CREATE TABLE authorization_audit_events (
    id UUID PRIMARY KEY,
    operator_id UUID NOT NULL REFERENCES operators(id),
    actor_identity_id UUID NOT NULL REFERENCES auth_identities(id),
    action TEXT NOT NULL CHECK (btrim(action) <> ''),
    target_type TEXT NOT NULL CHECK (btrim(target_type) <> ''),
    target_id TEXT NOT NULL CHECK (btrim(target_id) <> ''),
    occurred_at TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX authorization_audit_operator_time_idx
    ON authorization_audit_events (operator_id, occurred_at DESC, id DESC);
