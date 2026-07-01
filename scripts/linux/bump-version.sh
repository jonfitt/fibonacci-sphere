#!/usr/bin/env bash
# Bump project version (major/minor/patch), sync manifests, commit, and tag.
#
# Usage:
#   ./scripts/linux/bump-version.sh
#   ./scripts/linux/bump-version.sh patch
#   ./scripts/linux/bump-version.sh --no-commit minor
#
# VERSION is the single source of truth. Cargo.toml workspace members use
# version.workspace = true; this script syncs VERSION into Cargo.toml, Cargo.lock, and
# docs/description.md before creating tag vX.Y.Z.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/linux/sync-version.sh
source "${ROOT}/scripts/linux/sync-version.sh"

VERSION_FILE="${ROOT}/VERSION"
COMMIT=1
BUMP_KIND=""

usage() {
  cat <<'EOF'
Usage: bump-version.sh [--no-commit] [major|minor|patch]

Interactively choose a semver bump unless major, minor, or patch is given.
Updates VERSION, syncs Cargo.toml, Cargo.lock, and docs/description.md, commits, and tags vX.Y.Z.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-commit)
      COMMIT=0
      shift
      ;;
    major|minor|patch)
      BUMP_KIND="$1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "${VERSION_FILE}" ]]; then
  echo "error: missing ${VERSION_FILE}" >&2
  exit 1
fi

CURRENT="$(tr -d '[:space:]' < "${VERSION_FILE}")"
if [[ ! "${CURRENT}" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "error: invalid VERSION: ${CURRENT}" >&2
  exit 1
fi

MAJOR="${BASH_REMATCH[1]}"
MINOR="${BASH_REMATCH[2]}"
PATCH="${BASH_REMATCH[3]}"

if [[ -z "${BUMP_KIND}" ]]; then
  echo "Current version: ${CURRENT}"
  echo
  echo "Select bump type:"
  echo "  1) major (${MAJOR} -> $((MAJOR + 1)).0.0)"
  echo "  2) minor (${MAJOR}.${MINOR} -> ${MAJOR}.$((MINOR + 1)).0)"
  echo "  3) patch (${CURRENT} -> ${MAJOR}.${MINOR}.$((PATCH + 1)))"
  echo
  read -r -p "Enter 1, 2, or 3 [3]: " choice
  choice="${choice:-3}"
  case "${choice}" in
    1|major) BUMP_KIND="major" ;;
    2|minor) BUMP_KIND="minor" ;;
    3|patch) BUMP_KIND="patch" ;;
    *)
      echo "error: invalid choice: ${choice}" >&2
      exit 1
      ;;
  esac
fi

case "${BUMP_KIND}" in
  major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    ;;
  minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    ;;
  patch)
    PATCH=$((PATCH + 1))
    ;;
  *)
    echo "error: bump kind must be major, minor, or patch" >&2
    exit 1
    ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
TAG="v${NEW_VERSION}"

if git -C "${ROOT}" rev-parse "refs/tags/${TAG}" >/dev/null 2>&1; then
  echo "error: tag already exists: ${TAG}" >&2
  exit 1
fi

printf '%s\n' "${NEW_VERSION}" > "${VERSION_FILE}"
sync_version_repo_root "${ROOT}" >/dev/null

echo "Bumped version: ${CURRENT} -> ${NEW_VERSION}"

if [[ "${COMMIT}" -eq 1 ]]; then
  git -C "${ROOT}" add VERSION Cargo.toml Cargo.lock docs/description.md
  if ! git -C "${ROOT}" diff --cached --quiet; then
    git -C "${ROOT}" commit -m "Bump version to ${NEW_VERSION}"
  fi
  git -C "${ROOT}" tag -a "${TAG}" -m "Release ${TAG}"
  echo "Created commit and annotated tag ${TAG}."
  echo "Push with:"
  echo "  git push origin HEAD"
  echo "  git push origin ${TAG}"
else
  echo "Updated files only (--no-commit). Review changes, then commit and tag manually."
fi
