#!/usr/bin/env bash
# Extract the Keep a Changelog section body for a semver version (without leading v).
set -euo pipefail

version="${1:?usage: extract-changelog-section.sh <version> [changelog-file]}"
changelog="${2:-CHANGELOG.md}"

version="${version#v}"

if [[ ! -f "$changelog" ]]; then
  echo "changelog file not found: $changelog" >&2
  exit 1
fi

awk -v ver="$version" '
  $0 ~ "^## \\[" ver "\\]" { found=1; next }
  found && /^## / { exit }
  found { print }
' "$changelog"
