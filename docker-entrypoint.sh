#!/bin/sh
set -eu

echo "Starting Budget API..."

# Start the application
echo "Starting Budget API server..."
exec /app/budget "$@"
