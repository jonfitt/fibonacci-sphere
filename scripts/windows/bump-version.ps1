# Bump project version (major/minor/patch), sync manifests, commit, and tag.
# Usage:
#   .\scripts\windows\bump-version.ps1
#   .\scripts\windows\bump-version.ps1 patch
#   .\scripts\windows\bump-version.ps1 -NoCommit minor
param(
    [Parameter(Position = 0)]
    [ValidateSet("major", "minor", "patch")]
    [string]$BumpKind,
    [switch]$NoCommit
)

$ErrorActionPreference = "Stop"
. "$PSScriptRoot/version-lib.ps1"
Invoke-VersionBump -BumpKind $BumpKind -NoCommit:$NoCommit
