# Translaas SDK for Rust

Official Translaas client SDK for Rust (`translaas` on crates.io — **not published yet**).

| | |
|---|---|
| **Status** | Phase 2 client (`0.0.0`) — live HTTP, in-memory cache, offline file cache |
| **MSRV** | Rust **1.86+** |
| **License** | MIT |

Part of the [translaas-all](https://github.com/Mantelabs/translaas-all) umbrella workspace (local path `sdk/rust`).

## Implementation plan

Phased roadmap aligned to the .NET reference SDK (`Translaas.SDK`):

- [translaas-sdk-rust-implementation.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md)
- [translaas-sdk-dotnet-porting-reference.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-dotnet-porting-reference.md)
- [translaas-sdk-http-api-spec.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-http-api-spec.md)

Tracking issues: foundation [#1](https://github.com/Mantelabs/translaas-sdk-rust/issues/1), client transport [#4](https://github.com/Mantelabs/translaas-sdk-rust/issues/4), client read surface [#5](https://github.com/Mantelabs/translaas-sdk-rust/issues/5), in-memory cache [#7](https://github.com/Mantelabs/translaas-sdk-rust/issues/7), offline file cache [#8](https://github.com/Mantelabs/translaas-sdk-rust/issues/8), hybrid L1 cache [#9](https://github.com/Mantelabs/translaas-sdk-rust/issues/9), offline decorator [#10](https://github.com/Mantelabs/translaas-sdk-rust/issues/10), sync service [#11](https://github.com/Mantelabs/translaas-sdk-rust/issues/11).

## Caching

Configure in-memory caching on [`ClientBuilder`](src/client/builder.rs):

```rust
use translaas::cache::CacheMode;
use translaas::client::Client;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::builder()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .base_url("https://api.translaas.local")
    .cache_mode(CacheMode::Group) // recommended default in Go/.NET
    .build()?;
# Ok(())
# }
```

| `CacheMode` | Caches |
|-------------|--------|
| `None` | Nothing (default) |
| `Entry` | `get_entry` + `get_project_locales` |
| `Group` | `get_group` + `get_project_locales` |
| `Project` | `get_group`, `get_project`, + `get_project_locales` |

304 responses fall back to cached values when a provider is configured; empty 304 bodies never overwrite cache entries.

## Offline file cache

Enable the `offline` feature for on-disk caching (`translaas::cachefile`). Disk I/O is synchronous — use `tokio::task::spawn_blocking` from async code when needed.

```toml
[dependencies]
translaas = { version = "0.1", features = ["offline"] }
```

```rust
use translaas::cachefile::{FileProvider, Provider, SaveOptions};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let provider = FileProvider::new(".translaas-cache")?;

provider.save_project(
    "demo-project",
    "en",
    &translation_project,
    SaveOptions::new(),
)?;

if let Some(project) = provider.get_project("demo-project", "en")? {
    // use cached project payload
    let _group = project.get_group("common")?;
}
# Ok(())
# }
```

On-disk layout:

```text
{CacheDirectory}/
├── manifest.json
└── {sanitizedProjectId}/
    ├── locales.json
    └── {sanitizedLang}/
        └── project.json
```

Expired wrapper entries are treated as misses; corrupt JSON returns `OfflineCacheError`.

### Hybrid L1-over-L2

Wrap [`FileProvider`] with [`HybridProvider`] for an in-memory L1 (TTL + LRU per partition) over disk L2. Defaults: enabled, 30 minute TTL, 1000 entries per partition. L1 uses the [`lru`](https://docs.rs/lru) crate with explicit expiry (see [`HybridProvider`](src/cachefile/hybrid_provider.rs) rustdoc for notes on `moka` / `quick_cache` evaluation).

This is separate from HTTP in-memory caching in [`cache::MemoryProvider`](src/cache/memory.rs).

```rust
use translaas::cachefile::{FileProvider, HybridOptions, HybridProvider, Provider, SaveOptions};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let file = FileProvider::new(".translaas-cache")?;
let provider = HybridProvider::new(file, HybridOptions::default());

provider.save_project("demo-project", "en", &translation_project, SaveOptions::new())?;

// Second read hits L1 without disk I/O.
let _ = provider.get_project("demo-project", "en")?;

provider.clear_memory_cache(); // L2 unchanged
# Ok(())
# }
```

### Offline decorator (`CachingClient`)

Wrap a live [`Client`](src/client/mod.rs) (or any [`TranslaasClient`](src/client/trait.rs)) with [`CachingClient`](src/cachefile/caching_client.rs) for disk-first / API-first / cache-only reads:

```rust
use translaas::cachefile::{
    CachingClient, CachingOptions, FallbackMode, FileProvider, HybridOptions, HybridProvider,
};
use translaas::client::Client;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let inner = Client::builder()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .default_project_id("demo-project")
    .build()?;

let cache = HybridProvider::new(
    FileProvider::new(".translaas-cache")?,
    HybridOptions::default(),
);

let client = CachingClient::new(
    inner,
    cache,
    CachingOptions {
        fallback_mode: FallbackMode::CacheFirst,
        default_project_id: "demo-project".into(),
    },
)?;

let text = client
    .get_entry("common", "hello", "en", translaas::client::GetEntryOptions::new())
    .await?;
# Ok(())
# }
```

| `FallbackMode` | Order |
|----------------|--------|
| `CacheFirst` (default) | Disk → API on miss |
| `ApiFirst` | API → disk on network/API errors |
| `CacheOnly` | Disk only |

Intercepted reads: `get_entry`, `get_group`, `get_project`, `get_project_locales`. Passthrough (always inner): `get_offline_cache`, `report_missing_keys`, `validate_api_key`.

Offline entry resolution uses simplified plural rules (`n == 1` → `One`, else `Other`) and `{param}` substitution — not full CLDR parity with the live API.

For **keyless offline-only** deployments, pair `FallbackMode::CacheOnly` with [`OfflineStubClient`](src/cachefile/offline_stub.rs) after seeding disk:

```rust
use translaas::cachefile::{CachingClient, CachingOptions, FallbackMode, FileProvider, OfflineStubClient};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let cache = FileProvider::new(".translaas-cache")?;
let client = CachingClient::new(
    OfflineStubClient::new(),
    cache,
    CachingOptions {
        fallback_mode: FallbackMode::CacheOnly,
        default_project_id: "demo-project".into(),
    },
)?;
# Ok(())
# }
```

### Populating the cache (`SyncService`)

Use [`SyncService`](src/cachefile/sync_service.rs) with the **inner** [`Client`](src/client/mod.rs) — not [`CachingClient`](src/cachefile/caching_client.rs) — to pull translations into disk:

```rust
use std::sync::Arc;
use translaas::cachefile::{
    FileProvider, OfflineCacheOptions, SyncCallbacks, SyncService,
};
use translaas::client::Client;
use tokio_util::sync::CancellationToken;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let inner = Client::builder()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .default_project_id("demo-project")
    .build()?;

let cache = FileProvider::new(".translaas-cache")?;
let mut opts = OfflineCacheOptions::default_offline_cache_options();
opts.default_project_id = "demo-project".into();
opts.projects = vec!["demo-project".into()];

let sync = SyncService::new(inner, cache, opts, SyncCallbacks::default());
let cancel = CancellationToken::new();

sync.sync_project("demo-project", "en", &cancel).await?;

let svc = Arc::new(sync);
svc.start_background_sync(cancel.clone());
// … later: svc.stop_background_sync().await;
# Ok(())
# }
```

[`OfflineCacheOptions`](src/cachefile/offline_cache_options.rs) is the umbrella config (Go / .NET §4.3). Derive [`CachingOptions`](src/cachefile/caching_options.rs) via `opts.caching_options()?` when wrapping the same inner client for reads.

Optional callbacks (`SyncCallbacks`) mirror Go hooks; adapt to channels by forwarding [`SyncEvent`](src/cachefile/sync_events.rs) variants from `on_sync_*` handlers.

## Quick start (async)

```rust
use translaas::client::{Client, GetEntryOptions};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::builder()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .base_url("https://api.translaas.local")
    .build()?;

let text = client
    .get_entry("ui", "greeting", "en", GetEntryOptions::new())
    .await?;
println!("{text}");
# Ok(())
# }
```

## Cargo features

| Feature | Default | Purpose |
|---------|---------|---------|
| `cache` | yes | In-memory cache layer (`translaas::cache`) |
| `offline` | no | On-disk / hybrid cache (`translaas::cachefile`); implies `cache` |
| `service` | no | Convenience `t()` helper (`translaas::service`) |
| `axum` | no | Axum extractors / helpers; implies `service` |

```toml
# Coming in M1 — do not publish consumers against 0.0.0 yet
# [dependencies]
# translaas = "0.1"
```

## Development

Requires [Rust](https://rustup.rs/) 1.86+ and optionally [`just`](https://github.com/casey/just).

```powershell
# From sdk/rust (or this repository root)
just help
just check          # fmt-check + clippy + test + build
```

Equivalent `cargo` commands (what CI runs):

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo build --workspace --all-features
```

Coverage (local only; install [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) first):

```powershell
just coverage
# or: cargo llvm-cov --all-features --summary-only
```

A thin `Makefile` forwards to `just` for contributors who prefer `make help` / `make lint` / `make test` / `make coverage`.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for TDD expectations, commit style, and module boundaries.

## Samples

Runnable sample apps live in **[translaas-sdk-examples](https://github.com/acuencadev/translaas-sdk-examples)** under `rust/`, not in this library repository.

## CI

GitHub Actions runs on **Ubuntu** and **Windows**: format, clippy (`-D warnings`), tests (feature matrix), and build. MSRV is pinned to **1.86.0** in a dedicated job.
