# Apply VERSION to Cargo.toml and docs/description.md without bumping.
$ErrorActionPreference = "Stop"
. "$PSScriptRoot/version-lib.ps1"
$root = Get-RepoRoot
Set-Location $root
$version = Sync-ProjectVersion -Root $root
Write-Host "Synced version $version from VERSION."
