# Windows has no `sh` by default; keep Unix on POSIX CI/dev hosts.
[windows]
set shell := ["pwsh.exe", "-NoLogo", "-Command"]

[unix]
set shell := ["sh", "-cu"]

default:
    @just help

help:
    @echo "translaas SDK (MSRV 1.86) — targets: fmt, fmt-check, clippy, lint, test, test-integration, build, coverage, check, clean"
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
    cargo test --features integration,service --test live_api -- --nocapture

build:
    cargo build --features cache,offline,service,axum

coverage:
    cargo llvm-cov --features cache,offline,service,axum --summary-only

check: lint test build

clean:
    cargo clean
