# Contributing to Translaas SDK for Rust

Thank you for contributing. This repository implements the
[Translaas SDK HTTP contract](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-http-api-spec.md)
with behavioral parity to the [.NET reference SDK](https://github.com/acuencadev/Translaas.SDK)
and the [Go SDK](https://github.com/Mantelabs/translaas-sdk-go).

## Getting started

1. Fork and clone the repository (umbrella path: `sdk/rust` under `translaas-all`).
2. Install **Rust 1.86+** (`rustup`). CI pins MSRV **1.86.0** and also tests on stable.
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
| `tests/` | Crate-level tests (wiremock + live API under `tests/integration/`) | smoke + integration |
| `testdata/` | Golden fixtures | later issues |

Runnable sample apps live in **[translaas-sdk-examples](https://github.com/acuencadev/translaas-sdk-examples)** (`rust/`), not here.

See the [implementation plan](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md).

## Development guidelines

### Test-driven development

- Prefer tests before or alongside implementation for non-trivial behavior.
- Run `just test` (or the `cargo test` matrix in the README).
- Default CI does **not** call live Translaas APIs. Use `make test-integration` with `TRANSLAAS_API_KEY` for live checks (see `tests/integration/README.md`).

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

This crate uses [Semantic Versioning](https://semver.org/) with optional pre-release
suffixes (`-alpha`, `-beta`, `-rc`). The first crates.io release is **`0.4.0-beta`**
(Go SDK parity). Consumers should pin exact versions:

```bash
cargo add translaas@=0.4.0-beta --features service
```

## Releasing

Tag-driven releases use [`.github/workflows/release.yml`](./.github/workflows/release.yml) —
the same quality bar as CI, plus `cargo publish` to [crates.io](https://crates.io/crates/translaas)
and a GitHub Release whose body is extracted from `CHANGELOG.md`.

### GitHub secrets (maintainers)

| Secret | Required for release | Purpose |
|--------|----------------------|---------|
| `CARGO_REGISTRY_TOKEN` | **Yes** | [crates.io API token](https://crates.io/settings/tokens) with publish scope. Mapped to the `CARGO_REGISTRY_TOKEN` env var in the release workflow. |
| `GITHUB_TOKEN` | Automatic | Creates GitHub Releases (`contents: write` on the publish job). |
| `TRANSLAAS_API_KEY` | No | Optional manual [integration workflow](./.github/workflows/integration.yml) only. |
| `TRANSLAAS_BASE_URL` | No | Optional integration override. |

Never commit token values. PR CI does **not** require `CARGO_REGISTRY_TOKEN`.

### Release checklist

1. Merge PRs with user-visible notes under `[Unreleased]` in `CHANGELOG.md`.
2. Promote `[Unreleased]` into `## [x.y.z] - YYYY-MM-DD`.
3. Bump `version` in `Cargo.toml` to match (no leading `v`).
4. Merge to `main` and confirm CI is green.
5. (Optional) Run **Actions → Integration Tests → Run workflow** with live API secrets.
6. Validate locally:

   **Windows (PowerShell):**

   ```powershell
   just publish-dry-run
   just validate-release 0.4.0-beta
   just release-tag-dry-run
   # or:
   pwsh -File scripts/validate-release-version.ps1 0.4.0-beta
   pwsh -File scripts/create-release-tag.ps1 -DryRun
   ```

   **Linux / macOS / CI:**

   ```bash
   just publish-dry-run
   bash scripts/create-release-tag.sh --dry-run
   ```

7. Create and push the tag (triggers the release workflow):

   **Windows (PowerShell):**

   ```powershell
   just release-tag 0.4.0-beta
   # or: pwsh -File scripts/create-release-tag.ps1 0.4.0-beta
   ```

   **Linux / macOS:**

   ```bash
   bash scripts/create-release-tag.sh 0.4.0-beta
   ```

8. Verify the [GitHub Release](https://github.com/Mantelabs/translaas-sdk-rust/releases),
   [crates.io page](https://crates.io/crates/translaas), and docs.rs build.
9. Confirm consumer install resolves: `cargo add translaas@=0.4.0-beta --features service`.

### First-time crates.io setup

1. Confirm the crate name `translaas` is available on crates.io.
2. Create a publish-scoped API token and add it as `CARGO_REGISTRY_TOKEN` under
   **Settings → Secrets and variables → Actions**.
3. After the first successful publish, add team owners with `cargo owner add` if needed.

Trusted publishing (OIDC) may replace the API token in a follow-up; v1 uses `CARGO_REGISTRY_TOKEN`.
