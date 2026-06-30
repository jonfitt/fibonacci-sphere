#!/usr/bin/env bash
# Require the Rust CI workflow to pass before merging into main.
set -euo pipefail

REPO="${1:-jonfitt/fibonacci-sphere}"
BRANCH="${2:-main}"
CHECK_CONTEXT="Rust / build"

if ! command -v gh >/dev/null 2>&1; then
	echo "error: GitHub CLI (gh) is required. Install it, then run: gh auth login" >&2
	exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
	echo "error: gh is not authenticated. Run: gh auth login" >&2
	exit 1
fi

echo "Configuring branch protection for ${REPO}:${BRANCH}"
echo "Required status check: ${CHECK_CONTEXT}"

gh api \
	--method PUT \
	-H "Accept: application/vnd.github+json" \
	"repos/${REPO}/branches/${BRANCH}/protection" \
	--input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "checks": [
      { "context": "${CHECK_CONTEXT}" }
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": false
}
EOF

echo "Branch protection enabled for ${BRANCH}."
echo "Pull requests can merge only when '${CHECK_CONTEXT}' is green."
