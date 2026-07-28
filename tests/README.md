# Integration-style tests

Wiremock and in-process integration tests live in this directory (`*_test.rs`).

**Live API** tests are feature-gated under [`integration/`](./integration/README.md) (issue [#14](https://github.com/Mantelabs/translaas-sdk-rust/issues/14)). Run them with `make test-integration` when `TRANSLAAS_API_KEY` is set.

Default `cargo test` / `just test` / PR CI do **not** hit the network.
