# Require the Rust CI workflow to pass before merging into main.
param(
    [string]$Repo = "jonfitt/fibonacci-sphere",
    [string]$Branch = "main"
)

$ErrorActionPreference = "Stop"
$CheckContext = "Rust / build"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI (gh) is required. Install it, then run: gh auth login"
}

gh auth status *> $null
if ($LASTEXITCODE -ne 0) {
    throw "gh is not authenticated. Run: gh auth login"
}

Write-Host "Configuring branch protection for ${Repo}:${Branch}"
Write-Host "Required status check: ${CheckContext}"

$Body = @{
    required_status_checks = @{
        strict = $true
        checks = @(@{ context = $CheckContext })
    }
    enforce_admins = $false
    required_pull_request_reviews = $null
    restrictions = $null
    allow_force_pushes = $false
    allow_deletions = $false
    block_creations = $false
    required_conversation_resolution = $false
} | ConvertTo-Json -Depth 4 -Compress

$Body | gh api --method PUT -H "Accept: application/vnd.github+json" "repos/$Repo/branches/$Branch/protection" --input -

if ($LASTEXITCODE -ne 0) {
    throw "Failed to configure branch protection."
}

Write-Host "Branch protection enabled for ${Branch}."
Write-Host "Pull requests can merge only when '$CheckContext' is green."
