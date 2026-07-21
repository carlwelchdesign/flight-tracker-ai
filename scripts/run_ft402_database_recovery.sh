#!/usr/bin/env bash
set -euo pipefail

: "${FT402_POSTGRES_CONTAINER:?Set FT402_POSTGRES_CONTAINER to the isolated PostGIS container ID}"
if [[ "${FT402_ALLOW_DESTRUCTIVE_RECOVERY:-false}" != "true" ]]; then
  echo "FT-402 recovery drill requires FT402_ALLOW_DESTRUCTIVE_RECOVERY=true" >&2
  exit 2
fi

source_database="${FT402_SOURCE_DATABASE:-flight_tracker}"
restore_database="${FT402_RESTORE_DATABASE:-flight_tracker_ft402_restore}"
database_user="${FT402_DATABASE_USER:-flight_tracker}"
archive_path="/tmp/flight_tracker_ft402_recovery.dump"
marker_id="40200000-0000-4000-8000-000000000001"

now_ms() {
  python3 -c 'import time; print(time.monotonic_ns() // 1_000_000)'
}

if [[ "$restore_database" != *_ft402_restore || "$restore_database" == "$source_database" ]]; then
  echo "Restore database must be a distinct name ending in _ft402_restore" >&2
  exit 2
fi

cleanup() {
  docker exec "$FT402_POSTGRES_CONTAINER" \
    dropdb --if-exists --force --username "$database_user" "$restore_database" >/dev/null 2>&1 || true
  docker exec "$FT402_POSTGRES_CONTAINER" rm -f "$archive_path" >/dev/null 2>&1 || true
}
trap cleanup EXIT

cleanup
docker exec "$FT402_POSTGRES_CONTAINER" \
  psql --set ON_ERROR_STOP=1 --username "$database_user" --dbname "$source_database" \
  --command "INSERT INTO operators (id, code, display_name) VALUES ('$marker_id', 'FT402', 'Controlled recovery drill') ON CONFLICT (id) DO UPDATE SET display_name = EXCLUDED.display_name" \
  >/dev/null

source_counts="$(docker exec "$FT402_POSTGRES_CONTAINER" \
  psql --tuples-only --no-align --field-separator=, --username "$database_user" --dbname "$source_database" \
  --command "SELECT (SELECT COUNT(*) FROM operators WHERE id = '$marker_id'), (SELECT COUNT(*) FROM alerts), (SELECT COUNT(*) FROM alert_actions), (SELECT COUNT(*) FROM _sqlx_migrations WHERE success)")"

started_ms="$(now_ms)"
docker exec "$FT402_POSTGRES_CONTAINER" \
  pg_dump --format=custom --no-owner --username "$database_user" \
  --dbname "$source_database" --file "$archive_path"
docker exec "$FT402_POSTGRES_CONTAINER" \
  createdb --username "$database_user" "$restore_database"
docker exec "$FT402_POSTGRES_CONTAINER" \
  pg_restore --exit-on-error --no-owner --username "$database_user" \
  --dbname "$restore_database" "$archive_path"

restored_counts="$(docker exec "$FT402_POSTGRES_CONTAINER" \
  psql --tuples-only --no-align --field-separator=, --username "$database_user" --dbname "$restore_database" \
  --command "SELECT (SELECT COUNT(*) FROM operators WHERE id = '$marker_id'), (SELECT COUNT(*) FROM alerts), (SELECT COUNT(*) FROM alert_actions), (SELECT COUNT(*) FROM _sqlx_migrations WHERE success)")"
postgis_present="$(docker exec "$FT402_POSTGRES_CONTAINER" \
  psql --tuples-only --no-align --username "$database_user" --dbname "$restore_database" \
  --command "SELECT COUNT(*) FROM pg_extension WHERE extname = 'postgis'")"

if [[ "$source_counts" != "$restored_counts" || "$postgis_present" != "1" ]]; then
  echo "FT-402 restored database failed the row-count or PostGIS release gate" >&2
  exit 1
fi

finished_ms="$(now_ms)"
rto_ms="$((finished_ms - started_ms))"
echo "FT402_DATABASE_RECOVERY rpo_transactions=0 rto_ms=$rto_ms postgis=present row_counts_match=true result=passed"
