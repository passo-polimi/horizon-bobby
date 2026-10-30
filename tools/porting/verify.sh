#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo "         Horizon Mod Porting Verification Gate (CI)         "
echo "============================================================"

# Determina directory radice del repo
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# 1. Controllo formattazione Rust (rustfmt)
echo "[1/4] Checking code formatting (cargo fmt)..."
cargo fmt --check --package horizon-mod-api --package distant-horizons --package nvidium --package horizon-mod-harness

# 2. Controllo linter e warning (cargo clippy)
echo "[2/4] Running Clippy with warnings denied..."
cargo clippy --all-targets -p horizon-mod-api -p distant-horizons -p nvidium -p horizon-mod-harness -- -D warnings

# 3. Test di unità API e suite mod pilota
echo "[3/4] Running API and mod unit test suites..."
cargo test -p horizon-mod-api --test api_tests
cargo test -p nvidium --test nvidium_tests
cargo test -p distant-horizons --test dh_tests

# 4. Suite di testing differenziale Java <-> Rust
echo "[4/4] Running Java <-> Rust differential tests (difftest)..."
cargo test -p horizon-mod-harness --test difftest

echo "============================================================"
echo "  [SUCCESS] All checks and differential tests passed cleanly!"
echo "============================================================"

