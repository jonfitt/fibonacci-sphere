#!/usr/bin/env bash
# Read VERSION and sync it into Cargo.toml, Cargo.lock, and docs/description.md.
# Sourced by bump-version.sh and ci-check.sh, or run directly to apply VERSION without bumping.
set -euo pipefail

read_lockfile_version() {
  local lockfile="${1:?}"
  perl -ne '
    if (/^name = "fibonacci_sphere"$/) { $want = 1 }
    elsif ($want && /^version = "([^"]+)"/) { print $1; exit }
  ' "${lockfile}"
}

sync_lockfile_versions() {
  local lockfile="${1:?}"
  local version="${2:?}"
  perl -i -pe '
    if (/^name = "(?:fibonacci_sphere|fibonacci_sphere_gd|sphere_lattice_visualizer)"$/) { $want = 1 }
    elsif ($want && /^version = "/) {
      s/^version = "[^"]+"/version = "'"${version}"'"/;
      $want = 0;
    }
  ' "${lockfile}"
}

sync_version_repo_root() {
  local root="${1:?repo root required}"
  local version_file="${root}/VERSION"
  local cargo_toml="${root}/Cargo.toml"
  local cargo_lock="${root}/Cargo.lock"
  local description_md="${root}/docs/description.md"

  if [[ ! -f "${version_file}" ]]; then
    echo "error: missing ${version_file}" >&2
    return 1
  fi

  local version
  version="$(tr -d '[:space:]' < "${version_file}")"
  if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "error: VERSION must be semver major.minor.patch, got: ${version}" >&2
    return 1
  fi

  if [[ ! -f "${cargo_toml}" ]]; then
    echo "error: missing ${cargo_toml}" >&2
    return 1
  fi

  perl -0777 -i -pe \
    's/(\[workspace\.package\][^\[]*?^version = ")[^"]+(")/${1}'"${version}"'${2}/ms' \
    "${cargo_toml}"

  if [[ -f "${description_md}" ]]; then
    perl -0777 -i -pe \
      's/(name = "fibonacci_sphere"\r?\nversion = ")[^"]+(")/${1}'"${version}"'${2}/s' \
      "${description_md}"
  fi

  if [[ -f "${cargo_lock}" ]]; then
    sync_lockfile_versions "${cargo_lock}" "${version}"
  fi

  printf '%s' "${version}"
}

verify_version_sync() {
  local root="${1:?repo root required}"
  local version_file="${root}/VERSION"
  local cargo_toml="${root}/Cargo.toml"
  local cargo_lock="${root}/Cargo.lock"
  local description_md="${root}/docs/description.md"

  local version
  version="$(tr -d '[:space:]' < "${version_file}")"

  local cargo_version
  cargo_version="$(perl -0777 -ne 'if (/^\[workspace\.package\][^\[]*?^version = "([^"]+)"/ms) { print $1; exit }' "${cargo_toml}")"
  if [[ "${cargo_version}" != "${version}" ]]; then
    echo "error: VERSION (${version}) does not match Cargo.toml workspace.package.version (${cargo_version})" >&2
    echo "Run: ./scripts/linux/sync-version.sh" >&2
    return 1
  fi

  if [[ -f "${description_md}" ]]; then
    local doc_version
    doc_version="$(perl -0777 -ne 'if (/name = "fibonacci_sphere"\r?\nversion = "([^"]+)"/s) { print $1; exit }' "${description_md}")"
    if [[ "${doc_version}" != "${version}" ]]; then
      echo "error: VERSION (${version}) does not match docs/description.md example (${doc_version})" >&2
      echo "Run: ./scripts/linux/sync-version.sh" >&2
      return 1
    fi
  fi

  if [[ -f "${cargo_lock}" ]]; then
    local lock_version
    lock_version="$(read_lockfile_version "${cargo_lock}")"
    if [[ "${lock_version}" != "${version}" ]]; then
      echo "error: VERSION (${version}) does not match Cargo.lock fibonacci_sphere version (${lock_version})" >&2
      echo "Run: ./scripts/linux/sync-version.sh" >&2
      return 1
    fi
  fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  version="$(sync_version_repo_root "${ROOT}")"
  echo "Synced version ${version} from VERSION."
fi
