# Windows has no `sh` by default; keep Unix on POSIX CI/dev hosts.
[windows]
set shell := ["pwsh.exe", "-NoLogo", "-Command"]

[unix]
set shell := ["sh", "-cu"]

default:
    @just help

help:
    @echo "translaas SDK (MSRV 1.86) — targets: fmt, fmt-check, clippy, lint, test, test-integration, build, coverage, check, publish-dry-run, validate-release, release-tag-dry-run, clean"
    @echo "Features: cache (default), offline, service, axum"
    @echo "Samples: https://github.com/acuencadev/translaas-sdk-examples (rust/)"

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --features cache,offline,service,axum -- -D warnings

lint: fmt-check clippy

test:
    cargo test --features cache,offline,service,axum

# Live API integration tests (requires TRANSLAAS_API_KEY).
test-integration:
    cargo test --features integration,service --test live_api -- --test-threads=1 --nocapture

build:
    cargo build --features cache,offline,service,axum

coverage:
    cargo llvm-cov --features cache,offline,service,axum --summary-only

check: lint test build

publish-dry-run:
    cargo publish --dry-run --locked

[windows]
validate-release version:
    pwsh -NoProfile -File scripts/validate-release-version.ps1 {{version}}

release-tag-dry-run:
    pwsh -NoProfile -File scripts/create-release-tag.ps1 -DryRun

release-tag version:
    pwsh -NoProfile -File scripts/create-release-tag.ps1 {{version}}

[unix]
validate-release version:
    bash scripts/validate-release-version.sh {{version}}

release-tag-dry-run:
    bash scripts/create-release-tag.sh --dry-run

release-tag version:
    bash scripts/create-release-tag.sh {{version}}

clean:
    cargo clean
