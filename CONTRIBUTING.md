# Contributing to Translaas SDK for Rust

Thank you for contributing. This repository implements the
[Translaas SDK HTTP contract](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-http-api-spec.md)
with behavioral parity to the [.NET reference SDK](https://github.com/acuencadev/Translaas.SDK)
and the [Go SDK](https://github.com/Mantelabs/translaas-sdk-go).

## Getting started

1. Fork and clone the repository (umbrella path: `sdk/rust` under `translaas-all`).
2. Install **Rust 1.80+** (`rustup`). CI pins MSRV **1.80.0** and also tests on stable.
3. Install [`just`](https://github.com/casey/just) for local targets (`cargo install just` or `winget install Casey.Just`).
4. Create a feature branch: `feature/short-description` or `fix/short-description`.

```powershell
git checkout -b feature/my-change
just check
```

## Repository layout

| Path | Purpose | Cargo feature |
|------|---------|---------------|
| `src/models/` | DTOs, errors, request context | always |
| `src/http/` | URL/query helpers (internal) | always |
| `src/validate/` | Options validation (internal) | always |
| `src/client/` | HTTP client | always |
| `src/cache/` | In-memory cache | `cache` (default) |
| `src/cachefile/` | Offline file cache and sync | `offline` |
| `src/service/` | Convenience `t()` API | `service` |
| `src/axum/` | Optional Axum helpers | `axum` |
| `tests/` | Crate-level tests | smoke now; live API later |
| `testdata/` | Golden fixtures | later issues |

Runnable sample apps live in **[translaas-sdk-examples](https://github.com/acuencadev/translaas-sdk-examples)** (`rust/`), not here.

See the [implementation plan](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md).

## Development guidelines

### Test-driven development

- Prefer tests before or alongside implementation for non-trivial behavior.
- Run `just test` (or the `cargo test` matrix in the README).
- Default CI does **not** call live Translaas APIs.

### Rust conventions

- Prefer **safe Rust**; `unsafe` is forbidden at the crate root (`#![forbid(unsafe_code)]`).
- Keep the public façade thin; put helpers in private modules.
- Document Cargo features in `Cargo.toml` and the README.
- English only for identifiers, comments, errors, and docs.
- Run `just lint` before pushing.

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `test:` tests
- `chore:` tooling / maintenance
- `ci:` CI configuration
- `refactor:` code change without behavior change

Reference GitHub issues in the commit body or PR: `Closes #123`.

### Pull requests

- Keep PRs focused on a single issue or vertical slice.
- Link the tracking issue.
- Ensure CI passes (lint, test, build on Ubuntu and Windows; MSRV job).
- Update `CHANGELOG.md` under `[Unreleased]` for user-visible changes.

## Versioning

This crate uses [Semantic Versioning](https://semver.org/). Foundation is `0.0.0`
until M1 ships a usable client API.
