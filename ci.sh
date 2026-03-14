#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; exit 1; }
step() { echo -e "\n${YELLOW}>>>${NC} $1"; }

step "Format check"
cargo fmt --check || fail "cargo fmt --check"
pass "cargo fmt"

step "Clippy"
cargo clippy --workspace --all-targets -- -D warnings || fail "cargo clippy"
pass "cargo clippy"

step "Build"
cargo build --verbose || fail "cargo build"
pass "cargo build"

step "Tests"
cargo test --verbose || fail "cargo test"
pass "cargo test"

step "Dependency audit"
if command -v cargo-audit &>/dev/null; then
    cargo audit || fail "cargo audit"
    pass "cargo audit"
else
    echo -e "${YELLOW}[SKIP]${NC} cargo-audit not installed (run: cargo install cargo-audit)"
fi

echo -e "\n${GREEN}All CI checks passed.${NC}"
