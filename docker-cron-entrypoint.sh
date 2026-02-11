#!/bin/sh
set -eu

if [ -z "${CRON_TOKEN:-}" ]; then
  echo "CRON_TOKEN is required"
  exit 1
fi

CRON_SCHEDULE="${CRON_SCHEDULE:-*/15 * * * *}"
CRON_BASE_URL="${CRON_BASE_URL:-http://budget:8000}"
CRON_API_BASE_PATH="${CRON_API_BASE_PATH:-/api/v1}"
CRON_ENDPOINT="${CRON_BASE_URL%/}${CRON_API_BASE_PATH}/cron/generate-periods"

printf "%s /usr/bin/curl -fsS -X POST -H 'x-cron-token: %s' '%s' >> /proc/1/fd/1 2>> /proc/1/fd/2\n" \
  "$CRON_SCHEDULE" "$CRON_TOKEN" "$CRON_ENDPOINT" > /etc/crontabs/root

echo "Cron schedule: ${CRON_SCHEDULE}"
echo "Cron endpoint: ${CRON_ENDPOINT}"

exec crond -f -l 8
