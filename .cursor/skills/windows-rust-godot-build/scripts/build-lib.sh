#!/usr/bin/env bash
# Build the fibonacci_sphere core library (workspace root package).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=_cargo_win.sh
source "${SCRIPT_DIR}/_cargo_win.sh"

profile=()
if [[ "${1:-}" == "--release" ]]; then
	profile=(--release)
fi

run_cargo_win build "${profile[@]}"

root="$(repo_root)"
if [[ "${1:-}" == "--release" ]]; then
	echo "Built: ${root}/target/release/fibonacci_sphere.rlib"
else
	echo "Built: ${root}/target/debug/fibonacci_sphere.rlib"
fi
