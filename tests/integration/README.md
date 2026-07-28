# Translaas Rust SDK — live API integration tests

Live API integration tests for the `translaas` crate. Test **shape** follows the [Go SDK integration suite](https://github.com/Mantelabs/translaas-sdk-go/tree/main/tests/integration); **fixture ids** follow [translaas-sdk-examples](https://github.com/Mantelabs/translaas-sdk-examples) (`translaas-sdk-samples` project seeded in local Mantelabs Docker).

## Prerequisites

- A running Translaas delivery API (development environment)
- Valid API key with access to fixture project data

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TRANSLAAS_API_KEY` | **Yes** to run | — | Raw `X-Api-Key` value |
| `TRANSLAAS_BASE_URL` | No | `https://api.translaas.local` | API origin only (no `/api` or `/sdk` suffix). Go SDK defaults to `sdk-api.translaas.local`; override either way if needed. |
| `TRANSLAAS_DEFAULT_PROJECT` | No | `translaas-sdk-samples` | Project id for scoped reads ([sdk-examples](https://github.com/Mantelabs/translaas-sdk-examples) default). |

When `TRANSLAAS_API_KEY` is unset, tests are **skipped** (not failed).

When the API key is set but the host cannot be reached (DNS/TLS/connection failure), tests **soft-skip** with exit code 0 — this is not a test failure.

When the configured project or Go-style fixture data is missing, tests that need populated payloads **soft-skip** with a hint to set `TRANSLAAS_DEFAULT_PROJECT` (Mantelabs platform returns HTTP **404** for unknown SDK resources; Go/.NET fixture APIs often return **204** with empty bodies instead).

Default `cargo test`, `just test`, and PR CI **do not** enable the `integration` feature and never require these variables.

### Local Docker (`platform/translaas`)

Local Compose exposes one API origin for Admin (`/api/v1/...`) and SDK (`/sdk/v1/...`) routes. The default base URL is **`https://api.translaas.local`** (same as `TRANSLAAS_BASE_URL` in platform `.env.example`).

Integration tests accept self-signed TLS for local `.translaas.local` hosts (browser trust alone is not enough for Rust/`reqwest`).

```powershell
# After: docker compose --profile core up -d
$env:TRANSLAAS_API_KEY = "<your-sdk-api-key>"   # X-Api-Key header value, not Admin JWT
make test-integration
```

See [docker-https-setup.md](https://github.com/Mantelabs/translaas-all/blob/main/platform/translaas/docs/docker-https-setup.md) for hosts and TLS trust.

## Fixture data

Canonical strings live in [translaas-sdk-examples `translaas_sdk_samples_strings.csv`](https://github.com/Mantelabs/translaas-sdk-examples/blob/main/dotnet/docs/translaas_sdk_samples_strings.csv). Live tests default to:

| Field | Value |
|-------|-------|
| Project | `translaas-sdk-samples` |
| Group (simple entry) | `common` |
| Entry (simple) | `welcome.message` |
| Group (plural) | `messages` |
| Entry (plural) | `item` |
| Language | `en` (optional: `fr`, `es`) |

Example SDK URL (matches Postman):

`GET /sdk/v1/translations/text?project=translaas-sdk-samples&group=common&lang=en&entry=welcome.message`

**Note:** Go / .NET *integration test* READMEs still mention legacy `test-project` / `ui` / `button.save` for generic dev APIs. JS package integration tests use **mocked** `test-project` / `common` / `welcome`. Local Mantelabs Docker dogfoods **`translaas-sdk-samples`** instead.

Tests that require populated payloads **soft-skip** when the API returns empty containers (204) or when the Mantelabs platform returns **404** for a missing project/group/entry.

### API behavior

| Endpoint | Missing resource | Go/.NET fixture API | Mantelabs platform | Integration test |
|----------|------------------|---------------------|--------------------|------------------|
| `get_entry` | not found | 204 → entry key fallback | 404 | Accepts 204 fallback or 404 |
| `get_group` / `get_project` / `get_project_locales` | not found | 204 → empty container | 404 | Accepts empty container or 404 |
| Invalid API key | auth failure | 401/403 | 401/403 | `Error::Api` |

## Running locally

### Linux / macOS

```bash
export TRANSLAAS_API_KEY="your-api-key"
export TRANSLAAS_BASE_URL="https://api-dev.example.com"   # optional
make test-integration
```

### Windows (PowerShell)

```powershell
$env:TRANSLAAS_API_KEY = "your-api-key"
$env:TRANSLAAS_BASE_URL = "https://api-dev.example.com"   # optional
make test-integration
```

### Direct `cargo test`

```powershell
cargo test --features integration,service --test live_api -- --nocapture
```

## CI (optional)

The repository includes `.github/workflows/integration.yml` for manual runs via **workflow_dispatch**. Configure these secrets on the repo:

- `TRANSLAAS_API_KEY`
- `TRANSLAAS_BASE_URL` (optional)

PR CI does **not** require integration secrets.

## Security

Never commit API keys. Use environment variables or CI secrets only.
