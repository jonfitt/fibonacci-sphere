#!/usr/bin/env bash
# Local checks mirrored by .github/workflows/rust.yml (format, lint, build, test).
# Use when the repo and Rust toolchain both live on Linux or WSL with native cargo.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

echo "==> verify VERSION sync"
# shellcheck source=scripts/linux/sync-version.sh
source "${ROOT}/scripts/linux/sync-version.sh"
verify_version_sync "${ROOT}"

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --all-targets -- -D warnings

echo "==> cargo build"
cargo build --verbose

echo "==> cargo test"
cargo test --verbose

echo "All checks passed."
