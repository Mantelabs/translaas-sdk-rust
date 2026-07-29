# Create and push an annotated semver tag. Validates Cargo.toml and CHANGELOG first.
param(
    [Parameter(Position = 0)]
    [string]$Version,

    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $Root

if ($Version -eq '-DryRun' -or $Version -eq '--dry-run') {
    $DryRun = $true
    $Version = ''
}

if ([string]::IsNullOrEmpty($Version)) {
    $inUnreleased = $false
    foreach ($line in Get-Content -LiteralPath 'CHANGELOG.md') {
        if ($line -match '^## \[Unreleased\]') {
            $inUnreleased = $true
            continue
        }
        if ($inUnreleased -and $line -match '^## \[(.+?)\]') {
            $Version = $Matches[1]
            break
        }
    }
}

if ([string]::IsNullOrEmpty($Version)) {
    Write-Error 'Could not determine version. Pass as argument (e.g. 0.4.0-beta).'
    exit 1
}

$Version = $Version -replace '^v', ''
$Tag = "v$Version"

if ($Version -notmatch '^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$') {
    Write-Error "Invalid semver version: $Version"
    exit 1
}

& (Join-Path $PSScriptRoot 'validate-release-version.ps1') -Version $Version
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$notes = & (Join-Path $PSScriptRoot 'extract-changelog-section.ps1') -Version $Version
if ([string]::IsNullOrWhiteSpace(($notes | Out-String).Trim())) {
    Write-Error "No CHANGELOG section for [$Version]. Add ## [$Version] - YYYY-MM-DD first."
    exit 1
}

$remoteTags = git ls-remote --tags origin $Tag 2>$null
if ($remoteTags -match "refs/tags/$([regex]::Escape($Tag))") {
    Write-Output "Tag $Tag already exists on origin; skipping release tag."
    exit 0
}

if ($DryRun) {
    Write-Output "Dry run OK: would create and push tag $Tag"
    exit 0
}

git diff-index --quiet HEAD --
if ($LASTEXITCODE -ne 0) {
    Write-Warning 'Uncommitted changes present.'
}

git tag -a $Tag -m "Release $Tag"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

git push origin $Tag
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Output "Created and pushed release tag $Tag"
