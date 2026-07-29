#!/usr/bin/env bash
# Create and push an annotated semver tag. Validates Cargo.toml and CHANGELOG first.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

version=""
dry_run=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      dry_run=true
      shift
      ;;
    -h | --help)
      echo "Usage: scripts/create-release-tag.sh [<version>] [--dry-run]"
      echo "  version   SemVer without leading v (default: first section after [Unreleased])"
      echo "  --dry-run Validate CHANGELOG, Cargo.toml, and tag name without creating or pushing"
      exit 0
      ;;
    -*)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
    *)
      version="$1"
      shift
      ;;
  esac
done

if [[ -z "$version" ]]; then
  version="$(awk '
    /^## \[Unreleased\]/ { in_unreleased=1; next }
    in_unreleased && /^## \[/ {
      gsub(/^## \[|\].*$/, ""); print; exit
    }
  ' CHANGELOG.md)"
fi

if [[ -z "$version" ]]; then
  echo "Could not determine version. Pass as argument (e.g. 0.4.0-beta)." >&2
  exit 1
fi

version="${version#v}"
tag="v${version}"

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Invalid semver version: $version" >&2
  exit 1
fi

bash scripts/validate-release-version.sh "$version"

notes="$(bash scripts/extract-changelog-section.sh "$version")"
if [[ -z "$notes" ]]; then
  echo "No CHANGELOG section for [$version]. Add ## [$version] - YYYY-MM-DD first." >&2
  exit 1
fi

remote_tags="$(git ls-remote --tags origin "$tag" 2>/dev/null || true)"
if echo "$remote_tags" | grep -q "refs/tags/${tag}"; then
  echo "Tag $tag already exists on origin; skipping release tag."
  exit 0
fi

if [[ "$dry_run" == true ]]; then
  echo "Dry run OK: would create and push tag $tag"
  exit 0
fi

if ! git diff-index --quiet HEAD --; then
  echo "Warning: uncommitted changes present." >&2
fi

git tag -a "$tag" -m "Release $tag"
git push origin "$tag"
echo "Created and pushed release tag $tag"
