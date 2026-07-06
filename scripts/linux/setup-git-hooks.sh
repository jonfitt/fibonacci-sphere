#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

chmod +x "${ROOT}/.githooks/pre-commit" "${ROOT}/scripts/linux/ci-check.sh"
git config core.hooksPath .githooks

if [ ! -f "${ROOT}/.gitconfig" ]; then
	cp "${ROOT}/.gitconfig.example" "${ROOT}/.gitconfig"
	echo "Created .gitconfig from .gitconfig.example — edit name and email before committing."
fi
git config --local include.path ../.gitconfig

echo "Git hooks enabled (core.hooksPath=.githooks)."
echo "Local commit identity: $(git config user.email) <$(git config user.name)>."
echo "Pre-commit dispatches to scripts/linux/ or scripts/windows/ by platform."
