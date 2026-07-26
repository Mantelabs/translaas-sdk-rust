# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `translaas::client`: `ClientBuilder` / `Client` with `reqwest` + `rustls`, options
  validation, and async `get_entry` for `GET /sdk/v1/translations/text` (200 / 204 /
  304, `ApiError` envelope, timeout → 408). No caching in this release slice (#4).
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
