#!/bin/bash
set -e

echo "Starting Budget API..."

# Run database migrations using sqlx-cli
# This will automatically wait for the database to be ready and retry if needed
echo "Running database migrations..."
max_retries=30
retry_count=0

while [ $retry_count -lt $max_retries ]; do
    if sqlx migrate run --source /app/migrations 2>&1; then
        echo "Migrations completed successfully!"
        break
    fi

    retry_count=$((retry_count + 1))
    if [ $retry_count -lt $max_retries ]; then
        echo "Migration attempt $retry_count/$max_retries failed, retrying in 2 seconds..."
        sleep 2
    else
        echo "ERROR: Database migrations failed after $max_retries attempts"
        exit 1
    fi
done

# Start the application
echo "Starting Budget API server..."
exec /app/budget "$@"
