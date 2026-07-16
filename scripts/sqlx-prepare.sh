#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

CONTAINER=zayden-sqlx-prepare
PORT=55432
PG_IMAGE=postgres:18-bookworm
export DATABASE_URL="postgres://zayden:zayden@localhost:${PORT}/zayden"

if command -v docker >/dev/null 2>&1; then
    RUNTIME=docker
elif command -v podman >/dev/null 2>&1; then
    RUNTIME=podman
else
    echo "error: docker or podman is required" >&2
    exit 1
fi

cleanup() { "$RUNTIME" rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

cleanup
echo ">> starting throwaway Postgres ($PG_IMAGE) on port $PORT"
"$RUNTIME" run --rm -d --name "$CONTAINER" \
    -e POSTGRES_USER=zayden -e POSTGRES_PASSWORD=zayden -e POSTGRES_DB=zayden \
    -p "${PORT}:5432" "$PG_IMAGE" >/dev/null

echo ">> waiting for Postgres to accept connections"
for _ in $(seq 1 30); do
    if "$RUNTIME" exec "$CONTAINER" pg_isready -U zayden -d zayden >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done
[ "${ready:-0}" = 1 ] || { echo "error: Postgres did not become ready" >&2; exit 1; }

echo ">> running migrations"
sqlx migrate run

echo ">> regenerating .sqlx cache"
cargo sqlx prepare --workspace -- --features ssr

echo ">> done. Review and commit any changes under .sqlx/"
