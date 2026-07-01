# Sync VERSION into Cargo.toml and docs/description.md.
$ErrorActionPreference = "Stop"

function Get-RepoRoot {
    $root = git rev-parse --show-toplevel 2>$null
    if (-not $root) {
        throw "Not inside a git repository."
    }
    return $root
}

function Read-ProjectVersion {
    param([string]$Root)

    $versionFile = Join-Path $Root "VERSION"
    if (-not (Test-Path $versionFile)) {
        throw "Missing VERSION file at $versionFile"
    }

    $version = (Get-Content -Raw $versionFile).Trim()
    if ($version -notmatch '^\d+\.\d+\.\d+$') {
        throw "VERSION must be semver major.minor.patch, got: $version"
    }
    return $version
}

function Sync-ProjectVersion {
    param([string]$Root)

    $version = Read-ProjectVersion -Root $Root
    $cargoToml = Join-Path $Root "Cargo.toml"
    $descriptionMd = Join-Path $Root "docs/description.md"

    if (-not (Test-Path $cargoToml)) {
        throw "Missing Cargo.toml at $cargoToml"
    }

    $cargoText = Get-Content -Raw $cargoToml
    $cargoPattern = '(\[workspace\.package\][\s\S]*?^version = ")[^"]+(")'
    if ($cargoText -notmatch $cargoPattern) {
        throw "Could not find [workspace.package] version in Cargo.toml"
    }
    $cargoText = [regex]::Replace($cargoText, $cargoPattern, "`${1}$version`${2}", 1)
    Set-Content -NoNewline -Path $cargoToml -Value $cargoText

    if (Test-Path $descriptionMd) {
        $docText = Get-Content -Raw $descriptionMd
        $docPattern = '(name = "fibonacci_sphere"(?:\r?\n)version = ")[^"]+(")'
        if ($docText -match $docPattern) {
            $docText = [regex]::Replace($docText, $docPattern, "`${1}$version`${2}", 1)
            Set-Content -NoNewline -Path $descriptionMd -Value $docText
        }
    }

    return $version
}

function Test-ProjectVersionSync {
    param([string]$Root)

    $version = Read-ProjectVersion -Root $Root
    $cargoToml = Join-Path $Root "Cargo.toml"
    $descriptionMd = Join-Path $Root "docs/description.md"

    $cargoText = Get-Content -Raw $cargoToml
    if ($cargoText -match '(?ms)^\[workspace\.package\][\s\S]*?^version = "([^"]+)"') {
        $cargoVersion = $Matches[1]
    } else {
        throw "Could not find [workspace.package] version in Cargo.toml"
    }

    if ($cargoVersion -ne $version) {
        throw "VERSION ($version) does not match Cargo.toml workspace.package.version ($cargoVersion). Run scripts\windows\sync-version.cmd"
    }

    if (Test-Path $descriptionMd) {
        $docText = Get-Content -Raw $descriptionMd
        if ($docText -match 'name = "fibonacci_sphere"(?:\r?\n)version = "([^"]+)"') {
            $docVersion = $Matches[1]
            if ($docVersion -ne $version) {
                throw "VERSION ($version) does not match docs/description.md example ($docVersion). Run scripts\windows\sync-version.cmd"
            }
        }
    }
}

function Invoke-VersionBump {
    param(
        [string]$BumpKind,
        [switch]$NoCommit
    )

    $root = Get-RepoRoot
    Set-Location $root

    $current = Read-ProjectVersion -Root $root
    $parts = $current.Split(".")
    $major = [int]$parts[0]
    $minor = [int]$parts[1]
    $patch = [int]$parts[2]

    if (-not $BumpKind) {
        Write-Host "Current version: $current"
        Write-Host ""
        Write-Host "Select bump type:"
        Write-Host "  1) major ($major -> $($major + 1).0.0)"
        Write-Host "  2) minor ($major.$minor -> $major.$($minor + 1).0)"
        Write-Host "  3) patch ($current -> $major.$minor.$($patch + 1))"
        Write-Host ""
        $choice = Read-Host "Enter 1, 2, or 3 [3]"
        if (-not $choice) { $choice = "3" }
        switch ($choice) {
            "1" { $BumpKind = "major" }
            "2" { $BumpKind = "minor" }
            "3" { $BumpKind = "patch" }
            "major" { $BumpKind = "major" }
            "minor" { $BumpKind = "minor" }
            "patch" { $BumpKind = "patch" }
            default { throw "Invalid choice: $choice" }
        }
    }

    switch ($BumpKind) {
        "major" {
            $major += 1
            $minor = 0
            $patch = 0
        }
        "minor" {
            $minor += 1
            $patch = 0
        }
        "patch" {
            $patch += 1
        }
        default {
            throw "Bump kind must be major, minor, or patch"
        }
    }

    $newVersion = "$major.$minor.$patch"
    $tag = "v$newVersion"

    if (git tag -l $tag) {
        throw "Tag already exists: $tag"
    }

    Set-Content -NoNewline -Path (Join-Path $root "VERSION") -Value "$newVersion`n"
    $synced = Sync-ProjectVersion -Root $root
    if ($synced -ne $newVersion) {
        throw "Sync failed: expected $newVersion, got $synced"
    }

    Write-Host "Bumped version: $current -> $newVersion"

    if (-not $NoCommit) {
        git add VERSION Cargo.toml docs/description.md
        $staged = git diff --cached --name-only
        if ($staged) {
            git commit -m "Bump version to $newVersion"
        }
        git tag -a $tag -m "Release $tag"
        Write-Host "Created commit and annotated tag $tag."
        Write-Host "Push with:"
        Write-Host "  git push origin HEAD"
        Write-Host "  git push origin $tag"
    } else {
        Write-Host "Updated files only (-NoCommit). Review changes, then commit and tag manually."
    }
}

if ($MyInvocation.InvocationName -ne '.') {
    param(
        [ValidateSet("major", "minor", "patch")]
        [string]$BumpKind,
        [switch]$NoCommit,
        [switch]$SyncOnly,
        [switch]$VerifyOnly
    )

    $root = Get-RepoRoot
    Set-Location $root

    if ($VerifyOnly) {
        Test-ProjectVersionSync -Root $root
        Write-Host "VERSION is in sync."
        exit 0
    }

    if ($SyncOnly) {
        $version = Sync-ProjectVersion -Root $root
        Write-Host "Synced version $version from VERSION."
        exit 0
    }

    Invoke-VersionBump -BumpKind $BumpKind -NoCommit:$NoCommit
}
