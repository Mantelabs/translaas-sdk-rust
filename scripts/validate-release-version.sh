#!/usr/bin/env bash
# Fail when Cargo.toml package version does not match the release version (no leading v).
set -euo pipefail

version="${1:?usage: validate-release-version.sh <version>}"
version="${version#v}"

toml_version="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name=="translaas") | .version')"

if [[ -z "$toml_version" || "$toml_version" == "null" ]]; then
  echo "Could not read translaas version from Cargo.toml" >&2
  exit 1
fi

if [[ "$toml_version" != "$version" ]]; then
  echo "Version mismatch: expected Cargo.toml version [$toml_version] to match release [$version]" >&2
  exit 1
fi

echo "Release version OK: $version"
