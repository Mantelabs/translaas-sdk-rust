# Fail when Cargo.toml package version does not match the release version (no leading v).
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'

$Version = $Version -replace '^v', ''

$metadataJson = cargo metadata --no-deps --format-version 1
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}

$metadata = $metadataJson | ConvertFrom-Json
$tomlVersion = ($metadata.packages | Where-Object { $_.name -eq 'translaas' }).version

if ([string]::IsNullOrEmpty($tomlVersion)) {
    Write-Error 'Could not read translaas version from Cargo.toml'
    exit 1
}

if ($tomlVersion -ne $Version) {
    Write-Error "Version mismatch: expected Cargo.toml version [$tomlVersion] to match release [$Version]"
    exit 1
}

Write-Output "Release version OK: $Version"
