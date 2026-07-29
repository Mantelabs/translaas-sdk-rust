# Translaas SDK for Rust

Official Translaas client SDK for Rust ([`translaas` on crates.io](https://crates.io/crates/translaas)).

| | |
|---|---|
| **Status** | M4 parity beta (`0.4.0-beta`) — live HTTP, in-memory cache, offline file cache, `service`, axum |
| **MSRV** | Rust **1.86+** |
| **License** | MIT |

Part of the [translaas-all](https://github.com/Mantelabs/translaas-all) umbrella workspace (local path `sdk/rust`).

## Implementation plan

Phased roadmap aligned to the .NET reference SDK (`Translaas.SDK`):

- [translaas-sdk-rust-implementation.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md)
- [translaas-sdk-dotnet-porting-reference.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-dotnet-porting-reference.md)
- [translaas-sdk-http-api-spec.md](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-http-api-spec.md)

Tracking issues: foundation [#1](https://github.com/Mantelabs/translaas-sdk-rust/issues/1), client transport [#4](https://github.com/Mantelabs/translaas-sdk-rust/issues/4), client read surface [#5](https://github.com/Mantelabs/translaas-sdk-rust/issues/5), in-memory cache [#7](https://github.com/Mantelabs/translaas-sdk-rust/issues/7), offline file cache [#8](https://github.com/Mantelabs/translaas-sdk-rust/issues/8), hybrid L1 cache [#9](https://github.com/Mantelabs/translaas-sdk-rust/issues/9), offline decorator [#10](https://github.com/Mantelabs/translaas-sdk-rust/issues/10), sync service [#11](https://github.com/Mantelabs/translaas-sdk-rust/issues/11), convenience `t()` API [#12](https://github.com/Mantelabs/translaas-sdk-rust/issues/12).

## Installation

### Umbrella workspace (local path)

When developing from a `translaas-all` checkout:

```toml
[dependencies]
translaas = { path = "../../sdk/rust", features = ["service"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Enable additional layers with Cargo features: `offline`, `service`, `axum` (see [Cargo features](#cargo-features)).

### crates.io

Pin to a semver release (recommended for production):

```bash
cargo add translaas@=0.4.0-beta --features service
```

```toml
[dependencies]
translaas = { version = "=0.4.0-beta", features = ["service"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Enable additional layers with Cargo features: `offline`, `service`, `axum` (see [Cargo features](#cargo-features)).

- [crates.io/translaas](https://crates.io/crates/translaas)
- [docs.rs/translaas](https://docs.rs/translaas)

Requires Rust **1.86+** and an async runtime (Tokio recommended) to drive [`Client`](src/client/mod.rs) methods.

Maintainers: see [CONTRIBUTING.md § Releasing](./CONTRIBUTING.md#releasing).

Runnable sample apps live in the meta-repo under [`examples/rust/`](https://github.com/Mantelabs/translaas-all/tree/main/examples/rust) — not in this library repository.

## Documentation

| Resource | Purpose |
|----------|---------|
| [Rust SDK integration guide (KB)](https://github.com/Mantelabs/translaas-all/blob/main/.docs/kb/sdk-rust.md) | User-facing quickstart, env vars, feature overview |
| [Implementation plan](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md) | Phased roadmap and issue breakdown |
| [HTTP API spec](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-http-api-spec.md) | Delivery API wire contract |
| [Porting reference](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-dotnet-porting-reference.md) | Cross-language behavioral contract |
| [Go SDK README](../go/README.md) (meta-repo) | Closest systems-language peer |

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
translaas = { version = "=0.4.0-beta", features = ["offline"] }
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

### Convenience `t()` API (`service`)

Enable the `service` feature for a thin wrapper over `get_entry` with automatic language resolution:

```toml
[dependencies]
translaas = { version = "=0.4.0-beta", features = ["service"] }
```

```rust
use translaas::client::{Client, ClientBuilder};
use translaas::service::{
    DefaultLanguageProvider, LanguageContext, LanguageResolver, Service, ServiceOptions,
    TOptions,
};

# async fn example() -> Result<(), translaas::service::Error> {
let client = ClientBuilder::new()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .default_project_id("demo-project")
    .build()?;

let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")])?;
let service = Service::new(client, ServiceOptions {
    resolver: Some(resolver),
});

// Explicit language bypasses the resolver chain.
let text = service
    .t("common", "welcome", TOptions::new().lang("de"))
    .await?;

// Automatic resolution uses LanguageContext (Axum #13 will populate this per request).
let text = service
    .t(
        "common",
        "welcome",
        TOptions::new().language_context(LanguageContext::new().with_language("pt")),
    )
    .await?;
# let _ = text;
# Ok(())
# }
```

When no resolver is configured and no explicit language is set, `t()` returns [`NoLanguageError`](src/models/errors.rs) — never an HTTP/API error. Provider failures in the chain are skipped silently (Go logs warnings; Rust has no logging dependency on `service`).

Context cancellation before resolve (`context.Canceled` parity) is deferred until the client exposes an explicit cancel handle on `get_entry`.

## Axum integration (`axum` feature)

Enable the optional Axum helpers when building web apps:

```toml
[dependencies]
translaas = { version = "=0.4.0-beta", features = ["axum"] }
axum = "0.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Wire middleware once, then use the [`Translaas`](src/axum/extract.rs) extractor in handlers:

```rust
use std::sync::Arc;

use axum::{Router, routing::get, middleware::from_fn_with_state};
use translaas::axum::{middleware, translaas_middleware, MiddlewareOptions, Translaas};
use translaas::client::ClientBuilder;
use translaas::service::{
    DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

async fn welcome(Translaas(service): Translaas<translaas::client::Client>) -> String {
    service
        .t("ui", "welcome", TOptions::new())
        .await
        .unwrap_or_else(|err| err.to_string())
}

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = ClientBuilder::new()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .base_url(std::env::var("TRANSLAAS_BASE_URL").unwrap_or_else(|_| "https://api.translaas.local".into()))
    .build()?;

let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")])?;
let base = Service::new(client, ServiceOptions { resolver: Some(resolver) });
let state = Arc::new(middleware(MiddlewareOptions::with_base_service(base))?);

let app = Router::new()
    .route("/", get(welcome))
    .layer(from_fn_with_state(state.clone(), translaas_middleware))
    .with_state(state);
# let _ = app;
# Ok(())
# }
```

**Language resolution (defaults):** query `?lang=` → parsed `Accept-Language` → cookie `language`. Configure sources via [`MiddlewareOptions`](src/axum/middleware.rs). Route/path language uses an optional [`RouteLanguageFn`](src/axum/language.rs) callback.

Runnable sample: [`examples/rust/basic`](https://github.com/Mantelabs/translaas-all/tree/main/examples/rust/basic) (Axum sample planned).

### Security — XSS

Translation strings returned by the SDK are **not HTML-escaped**. When rendering HTML, escape at the template layer (`askama`, `maud`, etc.) rather than concatenating raw translation output into markup. JSON and API responses must be encoded at the serializer layer.

## Quick start

### Option A — `service::Service` (recommended)

```rust
use translaas::cache::CacheMode;
use translaas::client::ClientBuilder;
use translaas::service::{
    DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = ClientBuilder::new()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .base_url(std::env::var("TRANSLAAS_BASE_URL").unwrap_or_else(|_| "https://sdk-api.translaas.local".into()))
    .default_project_id(std::env::var("TRANSLAAS_DEFAULT_PROJECT").unwrap_or_else(|_| "test-project".into()))
    .cache_mode(CacheMode::Group)
    .build()?;

let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")])?;
let service = Service::new(client, ServiceOptions {
    resolver: Some(resolver),
});

let text = service
    .t("ui", "button.save", TOptions::new().lang("en"))
    .await?;
println!("{text}");
# Ok(())
# }
```

See also: [`examples/rust/basic`](https://github.com/Mantelabs/translaas-all/tree/main/examples/rust/basic).

### Option B — `client::Client` (direct API)

```rust
use translaas::client::{Client, GetEntryOptions};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::builder()
    .api_key(std::env::var("TRANSLAAS_API_KEY")?)
    .base_url("https://api.translaas.local")
    .default_project_id("test-project")
    .build()?;

let text = client
    .get_entry("ui", "greeting", "en", GetEntryOptions::new())
    .await?;
// 200 → plain text body; 204 → returns entry key unchanged
println!("{text}");
# Ok(())
# }
```

Additional options on [`GetEntryOptions`](src/client/get_entry.rs): number, parameters, request context. Group/project/locales endpoints return **JSON** payloads.

The text endpoint returns **plain text** (`Accept: text/plain`), **not** a JSON wrapper like `{ "value": "…" }`.

## Compatibility

| Rust SDK | .NET SDK | Go SDK | Delivery API | Notes |
|----------|----------|--------|--------------|-------|
| `0.4.0-beta` | `v0.4.1-beta` | `v0.4.0-beta` | `/sdk/v1` + `/api/v1/api-keys/validate` | M4 parity: client, cache, offline, `t()`, axum |
| (future) `v0.3.0-beta` | — | `v0.3.0-beta` | same | Offline + sync |
| (future) `v0.2.0-beta` | — | `v0.2.0-beta` | same | In-memory `CacheMode` |
| (future) `v0.1.0-alpha` | — | `v0.1.0-alpha` | same | Read-only client |

**Known divergences:** no built-in retry policy in Rust v1; simplified offline pluralization; text endpoint returns plain text (not JSON).

## Cargo features

| Feature | Default | Purpose |
|---------|---------|---------|
| `cache` | yes | In-memory cache layer (`translaas::cache`) |
| `offline` | no | On-disk / hybrid cache (`translaas::cachefile`); implies `cache` |
| `service` | no | Convenience `t()` helper (`translaas::service`) |
| `axum` | no | Axum extractors / helpers; implies `service` |
| `integration` | no | **Test-only** — live API integration harness (`make test-integration`) |

```toml
[dependencies]
translaas = { version = "=0.4.0-beta", features = ["cache"] }
```

## Development

Requires [Rust](https://rustup.rs/) 1.86+ and optionally [`just`](https://github.com/casey/just).

```powershell
# From sdk/rust (or this repository root)
just help
just check          # fmt-check + clippy + test + build
make test-integration   # live API (requires TRANSLAAS_API_KEY)
```

Live integration setup: [`tests/integration/README.md`](./tests/integration/README.md).

Equivalent `cargo` commands (what CI runs):

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --workspace --features cache,offline,service,axum
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

Runnable sample apps live in the meta-repo under [`examples/rust/`](https://github.com/Mantelabs/translaas-all/tree/main/examples/rust) — not in this library repository.

| Sample | Status | Purpose |
|--------|--------|---------|
| [`basic/`](https://github.com/Mantelabs/translaas-all/tree/main/examples/rust/basic) | Available | Console quickstart — fetch a translation with `service::t()` |
| `offline/` | Planned | Sync project to disk, then read with cache-only mode |
| `axum/` | Planned | Axum middleware + extractor |

## CI

GitHub Actions runs on **Ubuntu** and **Windows**: format, clippy (`-D warnings`), tests (feature matrix), and build. MSRV is pinned to **1.86.0** in a dedicated job.

Tag-driven releases use [`.github/workflows/release.yml`](./.github/workflows/release.yml) — the same quality bar as CI, plus `cargo publish` to crates.io and a GitHub Release from `CHANGELOG.md`. See [CONTRIBUTING.md § Releasing](./CONTRIBUTING.md#releasing).
