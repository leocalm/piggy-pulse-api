#!/bin/sh
set -eu

DB_URL="${PIGGY_PULSE_DATABASE__URL:-${DATABASE_URL:-}}"
if [ -z "${DB_URL}" ]; then
  echo "PIGGY_PULSE_DATABASE__URL (or DATABASE_URL) is required"
  exit 1
fi

CRON_SCHEDULE="${CRON_SCHEDULE:-*/15 * * * *}"
CRON_FILE="/etc/cron.d/piggy-pulse-api-generate-periods"

{
  echo "SHELL=/bin/sh"
  echo "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
  echo "TZ=${TZ:-UTC}"
  echo "PIGGY_PULSE_DATABASE__URL=${DB_URL}"
  [ -n "${PIGGY_PULSE_DATABASE__MAX_CONNECTIONS:-}" ] && echo "PIGGY_PULSE_DATABASE__MAX_CONNECTIONS=${PIGGY_PULSE_DATABASE__MAX_CONNECTIONS}"
  [ -n "${PIGGY_PULSE_DATABASE__MIN_CONNECTIONS:-}" ] && echo "PIGGY_PULSE_DATABASE__MIN_CONNECTIONS=${PIGGY_PULSE_DATABASE__MIN_CONNECTIONS}"
  [ -n "${PIGGY_PULSE_DATABASE__CONNECTION_TIMEOUT:-}" ] && echo "PIGGY_PULSE_DATABASE__CONNECTION_TIMEOUT=${PIGGY_PULSE_DATABASE__CONNECTION_TIMEOUT}"
  [ -n "${PIGGY_PULSE_DATABASE__ACQUIRE_TIMEOUT:-}" ] && echo "PIGGY_PULSE_DATABASE__ACQUIRE_TIMEOUT=${PIGGY_PULSE_DATABASE__ACQUIRE_TIMEOUT}"
  [ -n "${PIGGY_PULSE_LOGGING__LEVEL:-}" ] && echo "PIGGY_PULSE_LOGGING__LEVEL=${PIGGY_PULSE_LOGGING__LEVEL}"
  [ -n "${PIGGY_PULSE_LOGGING__JSON_FORMAT:-}" ] && echo "PIGGY_PULSE_LOGGING__JSON_FORMAT=${PIGGY_PULSE_LOGGING__JSON_FORMAT}"
  echo "${CRON_SCHEDULE} root /app/cron generate-periods >> /proc/1/fd/1 2>> /proc/1/fd/2"
} > "${CRON_FILE}"

chmod 0644 "${CRON_FILE}"

echo "Cron schedule: ${CRON_SCHEDULE}"
echo "Cron job: /app/cron generate-periods"

exec cron -f
