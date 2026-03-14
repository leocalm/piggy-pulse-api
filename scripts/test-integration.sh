#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DB_URL="postgres://postgres:test_password@127.0.0.1:5433/piggy_pulse_test"

step() { echo -e "\n${YELLOW}>>>${NC} $1"; }
pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; }

cleanup() {
    step "Stopping test database..."
    docker compose -f docker-compose.test.yaml down -v 2>/dev/null || true
}
trap cleanup EXIT

step "Starting test database..."
docker compose -f docker-compose.test.yaml up -d --wait

step "Running V2 integration tests..."
if DATABASE_URL="$DB_URL" cargo test --test 'v2_*' -- --ignored "$@"; then
    pass "All V2 integration tests passed"
else
    fail "Some V2 integration tests failed"
    exit 1
fi

echo -e "\n${GREEN}All V2 integration tests completed.${NC}"
