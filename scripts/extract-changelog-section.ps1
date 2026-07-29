# Extract the Keep a Changelog section body for a semver version (without leading v).
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version,

    [Parameter(Position = 1)]
    [string]$ChangelogPath = 'CHANGELOG.md'
)

$ErrorActionPreference = 'Stop'

$Version = $Version -replace '^v', ''

if (-not (Test-Path -LiteralPath $ChangelogPath)) {
    Write-Error "changelog file not found: $ChangelogPath"
    exit 1
}

$headingPattern = "^## \[$([regex]::Escape($Version))\]"
$found = $false

Get-Content -LiteralPath $ChangelogPath | ForEach-Object {
    if (-not $found -and $_ -match $headingPattern) {
        $found = $true
        return
    }
    if ($found -and $_ -match '^## ') {
        break
    }
    if ($found) {
        $_
    }
}
