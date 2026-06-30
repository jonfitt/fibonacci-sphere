#!/usr/bin/env bash
# Build fibonacci_sphere_gd for Godot (Windows DLL by default in this setup).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=_cargo_win.sh
source "${SCRIPT_DIR}/_cargo_win.sh"

profile=()
if [[ "${1:-}" == "--release" ]]; then
	profile=(--release)
fi

run_cargo_win build -p fibonacci_sphere_gd "${profile[@]}"

root="$(repo_root)"
if [[ "${1:-}" == "--release" ]]; then
	echo "Built: ${root}/target/release/fibonacci_sphere_gd.dll"
else
	echo "Built: ${root}/target/debug/fibonacci_sphere_gd.dll"
fi
