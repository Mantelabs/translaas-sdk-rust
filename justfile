default:
    @just help

help:
    @echo "translaas SDK (MSRV 1.80) — targets: fmt, fmt-check, clippy, lint, test, build, coverage, check, clean"
    @echo "Features: cache (default), offline, service, axum"
    @echo "Samples: https://github.com/acuencadev/translaas-sdk-examples (rust/)"

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

lint: fmt-check clippy

test:
    cargo test --all-features

build:
    cargo build --all-features

coverage:
    cargo llvm-cov --all-features --summary-only

check: lint test build

clean:
    cargo clean
