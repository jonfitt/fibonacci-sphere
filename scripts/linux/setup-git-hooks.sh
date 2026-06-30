#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

chmod +x "${ROOT}/.githooks/pre-commit" "${ROOT}/scripts/linux/ci-check.sh"
git config core.hooksPath .githooks

echo "Git hooks enabled (core.hooksPath=.githooks)."
echo "Pre-commit dispatches to scripts/linux/ or scripts/windows/ by platform."
