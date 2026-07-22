#!/usr/bin/env bash
set -euo pipefail

: "${TEST_DATABASE_URL:?TEST_DATABASE_URL is required}"

clerk_org_id="ft404_ci_organization"
clerk_user_id="ft404_ci_reviewer"

for _ in 1 2; do
  psql "$TEST_DATABASE_URL" \
    --set="clerk_org_id=$clerk_org_id" \
    --set="clerk_user_id=$clerk_user_id" \
    --file=scripts/bootstrap_hosted_portfolio.sql \
    >/dev/null
done

result="$(psql "$TEST_DATABASE_URL" --tuples-only --no-align --command="
  SELECT
    (SELECT COUNT(*) FROM operators
      WHERE id = '9c704a09-a62c-43d5-bac6-94ea2fd53b32'
        AND identity_provider = 'clerk'
        AND external_tenant_id = '$clerk_org_id'),
    (SELECT COUNT(*) FROM auth_identities
      WHERE provider = 'clerk' AND subject = '$clerk_user_id'
        AND disabled_at IS NULL),
    (SELECT COUNT(*)
       FROM operator_memberships membership
       JOIN auth_identities identity ON identity.id = membership.identity_id
      WHERE membership.operator_id = '9c704a09-a62c-43d5-bac6-94ea2fd53b32'
        AND identity.provider = 'clerk'
        AND identity.subject = '$clerk_user_id'
        AND membership.role = 'administrator'
        AND membership.status = 'active');
")"

test "$result" = "1|1|1"
printf 'FT-404 hosted portfolio bootstrap is idempotent and tenant-bound.\n'
