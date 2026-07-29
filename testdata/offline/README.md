# Offline ZIP test fixtures

Golden archives for [`cachefile`](../tests/cachefile_zip_bundle_test.rs) integration tests.

## Layout (HTTP spec §7.6)

```text
manifest.json
{project}/locales.json
{project}/{locale}/project.json
```

## Files

| File | Purpose |
|------|---------|
| `demo-project-bundle.zip` | Stable bundle with `demo-project` (`en`, `de`) |

## Regenerate

From `sdk/rust`:

```powershell
cargo test --features offline --test cachefile_zip_bundle_test write_golden_offline_fixture -- --ignored --exact
```

Or rebuild with the shared helper in `tests/support/offline_zip.rs` (Go-parity inline builder).
