# Enable git hooks for this clone.
$ErrorActionPreference = "Stop"

$Root = git rev-parse --show-toplevel
if (-not $Root) {
    throw "Not inside a git repository."
}
Set-Location $Root

git config core.hooksPath .githooks

Write-Host "Git hooks enabled (core.hooksPath=.githooks)."
Write-Host "Pre-commit dispatches to scripts/linux/ or scripts/windows/ by platform."
