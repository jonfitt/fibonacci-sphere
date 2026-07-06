# Enable git hooks for this clone.
$ErrorActionPreference = "Stop"

$Root = git rev-parse --show-toplevel
if (-not $Root) {
    throw "Not inside a git repository."
}
Set-Location $Root

git config core.hooksPath .githooks

$GitConfig = Join-Path $Root ".gitconfig"
$GitConfigExample = Join-Path $Root ".gitconfig.example"
if (-not (Test-Path $GitConfig)) {
    Copy-Item $GitConfigExample $GitConfig
    Write-Host "Created .gitconfig from .gitconfig.example - edit name and email before committing."
}
git config --local include.path ../.gitconfig

Write-Host "Git hooks enabled (core.hooksPath=.githooks)."
Write-Host "Local commit identity: $((git config user.email)) ($((git config user.name)))."
Write-Host "Pre-commit dispatches to scripts/linux/ or scripts/windows/ by platform."
