# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- MSRV raised to **1.86** (crates.io dependencies now require edition 2024 and ICU 2.2 / Rust ≥ 1.86).

### Added

- `translaas::service`: convenience `t()` API with `LanguageResolver`, built-in language
  providers (`DefaultLanguageProvider`, `ContextLanguageProvider`, `AcceptLanguageProvider`),
  `LanguageContext`, and `with_prepended_providers` (#12).
- Service integration tests (Go-parity mock matrix for resolver order, explicit lang bypass,
  option forwarding, prepend).
- `translaas::cachefile::SyncService`: offline cache synchronization with inner
  client, language filter, sync-all partial aggregation, optional background sync
  (`start_background_sync` / `stop_background_sync`), and `SyncCallbacks` hooks (#11).
- `translaas::cachefile::OfflineCacheOptions`: umbrella offline config with
  `caching_options()` bridge to `CachingOptions`.
- Sync service integration tests (Go-parity mock matrix, FileProvider round-trip,
  background cancel/stop).
- `translaas::cachefile::CachingClient`: offline decorator with `CacheFirst`, `ApiFirst`, and
  `CacheOnly` fallback modes; offline plural + placeholder resolution; group cache warm after
  API entry reads; `OfflineStubClient` for keyless `CacheOnly` (#10).
- CachingClient integration tests (mock + FileProvider, fault injection, concurrent reads).
- `translaas::cachefile::HybridProvider`: expirable LRU memory L1 over any L2
  `Provider` with promotion on read, dual-write on save, warmup, and stats (#9).
  L1 uses the `lru` crate with explicit TTL; `moka` and `quick_cache` were
  evaluated (documented on `HybridProvider`).
- Hybrid provider integration tests (promotion, TTL, LRU, concurrency, FileProvider).
- `translaas::cachefile`: `FileProvider` on-disk offline cache with JSON wrappers,
  root `manifest.json`, path sanitization, atomic `*.tmp` writes, expiration-as-miss,
  and corrupt JSON → `OfflineCacheError` (#8).
- Offline file provider integration tests (round-trip, expiry, manifest fallbacks).
- `translaas::client`: in-memory cache integration via `CacheMode`, `cache_ttl`, and
  injectable `MemoryProvider` on `ClientBuilder`; read methods honor the Go/.NET mode
  matrix; 304 responses fall back to cache without poisoning (#7).
- Wiremock cache integration tests (hit/miss, 304, TTL expiry, validate passthrough).
- `translaas::cache`: `CacheMode`, byte-identical `KeyBuilder` keys, typed `Provider`
  trait, and thread-safe `MemoryProvider` with absolute/sliding TTL, LRU eviction,
  and optional statistics (#6).
- Golden cache key fixtures under `testdata/cache_keys.json` (aligned with Go SDK).
- `translaas::client`: `TranslaasClient` trait plus `get_group`, `get_project`,
  `get_project_locales`, `get_offline_cache`, `report_missing_keys`, and
  `validate_api_key`; `ClientBuilder::build_with_resolved_project` for single-project
  API keys (#5).
- Wiremock integration tests for JSON read/write endpoints, offline ZIP metadata,
  bootstrap, and trait delegation.
- `translaas::client`: `ClientBuilder` / `Client` with `reqwest` + `rustls`, options
  validation, and async `get_entry` for `GET /sdk/v1/translations/text` (200 / 204 /
  304, `ApiError` envelope, timeout → 408) (#4).
- Wiremock integration tests for client success, error, and timeout paths.
- Internal `http` module (crate-private): base URL join/validation, request DTO query
  encoding, extra-parameter merge, and capital-`N` plural injection for client use in #4.
- Golden URL/query fixture `testdata/urls.json` aligned with Go `internal/httpx`.
- `translaas::models` module: typed errors, `RequestContext`, request/response DTOs,
  dual-shape `TranslationGroup`, flexible `TranslationProject`, plural categories,
  language code constants, and API key validation helpers.
- Golden JSON fixtures under `testdata/` for models parity with Go/.NET SDKs.

### Added (foundation)

- Repository foundation: crate layout, features map, rustfmt/clippy, justfile,
  GitHub Actions CI (Ubuntu + Windows), and contributor docs.
