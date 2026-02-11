#!/bin/sh
set -eu

DB_URL="${BUDGET_DATABASE__URL:-${DATABASE_URL:-}}"
if [ -z "${DB_URL}" ]; then
  echo "BUDGET_DATABASE__URL (or DATABASE_URL) is required"
  exit 1
fi

CRON_SCHEDULE="${CRON_SCHEDULE:-*/15 * * * *}"
CRON_FILE="/etc/cron.d/budget-generate-periods"

{
  echo "SHELL=/bin/sh"
  echo "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
  echo "TZ=${TZ:-UTC}"
  echo "BUDGET_DATABASE__URL=${DB_URL}"
  [ -n "${BUDGET_DATABASE__MAX_CONNECTIONS:-}" ] && echo "BUDGET_DATABASE__MAX_CONNECTIONS=${BUDGET_DATABASE__MAX_CONNECTIONS}"
  [ -n "${BUDGET_DATABASE__MIN_CONNECTIONS:-}" ] && echo "BUDGET_DATABASE__MIN_CONNECTIONS=${BUDGET_DATABASE__MIN_CONNECTIONS}"
  [ -n "${BUDGET_DATABASE__CONNECTION_TIMEOUT:-}" ] && echo "BUDGET_DATABASE__CONNECTION_TIMEOUT=${BUDGET_DATABASE__CONNECTION_TIMEOUT}"
  [ -n "${BUDGET_DATABASE__ACQUIRE_TIMEOUT:-}" ] && echo "BUDGET_DATABASE__ACQUIRE_TIMEOUT=${BUDGET_DATABASE__ACQUIRE_TIMEOUT}"
  [ -n "${BUDGET_LOGGING__LEVEL:-}" ] && echo "BUDGET_LOGGING__LEVEL=${BUDGET_LOGGING__LEVEL}"
  [ -n "${BUDGET_LOGGING__JSON_FORMAT:-}" ] && echo "BUDGET_LOGGING__JSON_FORMAT=${BUDGET_LOGGING__JSON_FORMAT}"
  echo "${CRON_SCHEDULE} root /app/cron generate-periods >> /proc/1/fd/1 2>> /proc/1/fd/2"
} > "${CRON_FILE}"

chmod 0644 "${CRON_FILE}"

echo "Cron schedule: ${CRON_SCHEDULE}"
echo "Cron job: /app/cron generate-periods"

exec cron -f
