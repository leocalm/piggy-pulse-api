#!/bin/sh
set -eu

echo "Starting PiggyPulse API..."

# Start the application
echo "Starting PiggyPulse API server..."
exec /app/piggy-pulse "$@"
