#!/usr/bin/env bash
# Run the full fibonacci_sphere workspace test suite via Windows cargo from WSL.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=_cargo_win.sh
source "${SCRIPT_DIR}/_cargo_win.sh"

run_cargo_win test "$@"
