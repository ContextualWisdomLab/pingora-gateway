# Pingora Gateway

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/pingora-gateway)

**A shared Rust edge runtime for ContextualWisdomLab services that need a small, explicit reverse-proxy boundary.**

Pingora Gateway centralizes edge concerns that should not be reimplemented independently by every product: upstream connection handling, bounded transport policy, request-size and in-flight limits, health/readiness, coarse telemetry, and container hardening. It is built on Cloudflare Pingora while exposing its own reviewed configuration contract to consumers instead of leaking Pingora types into product APIs.

> This README describes the current candidate branch. Protected `main` remains shipped authority until this Draft satisfies current review, security, supply-chain, and integration governance.

## Why it exists

Shared infrastructure is useful only when it reduces duplication **without absorbing product authority**. Pingora Gateway therefore owns the reusable HTTP edge boundary and deliberately leaves business routing, product authentication, identity, certificate authority, workflow state, and domain-specific retry/failover policy with their owning products.

| Need | What the current gateway provides |
| --- | --- |
| Edge runtime | Executable Rust/Pingora reverse-proxy path |
| Explicit upstream | One reviewed HTTP or HTTPS upstream in the v1 configuration contract |
| Bounded I/O | Connect/read/write/idle, request-body, process in-flight, and upstream keepalive-pool budgets |
| Backpressure | Fail-fast HTTP 503 when the configured non-health in-flight budget is exhausted; health remains observable |
| Forwarding safety | Distrust of client-supplied forwarding identity and hop-by-hop header policy |
| Operations | `/livez`, `/readyz`, low-cardinality Prometheus metrics, and coarse credential-safe access logs |
| Container boundary | Non-root runtime, read-only-root compatibility, dropped Linux capabilities, and `no-new-privileges` contract |
| Reproducibility | Locked Rust dependency graph and immutable Pingora git revision |

## Product boundary

```text
Client traffic
      │
      ▼
┌─────────────────────────────┐
│       Pingora Gateway       │
│ shared transport / edge     │
└──────────────┬──────────────┘
               │
         explicit upstream
               │
               ▼
         product service
```

Pingora Gateway does **not** own product authentication, tenant/business routing, application authorization, certificate issuance, static-site semantics, Wardnet/EgressWeave policy, Keyverse identity, or workflow state. Product-specific replay safety and failover stay outside this shared boundary because the gateway cannot infer domain idempotency.

## Quickstart

The current crate is `cwl-pingora-gateway` `0.1.0`, requires Rust 1.98.1 or newer, and pins Pingora `0.8.0` to an immutable upstream revision. Release-producing CI and image builds select Rust 1.98.1 exactly until a later compiler is separately reviewed.

Copy the example configuration, point the single upstream at a service you control, choose explicit positive `max_in_flight_requests` and `upstream_keepalive_pool_size` budgets, and run the gateway:

```bash
cp examples/gateway.yaml ./gateway.yaml
cargo run --locked --bin cwl-pingora-gateway -- --config ./gateway.yaml
```

The configuration is fail-closed: unknown fields, unsupported versions, listener collisions, zero body/concurrency/keepalive budgets or timeouts, multiple v1 upstreams, and incomplete TLS identity are rejected rather than normalized into a guessed configuration.

Check the local process separately for liveness and readiness:

```bash
curl -sS http://127.0.0.1:<traffic-port>/livez
curl -sS http://127.0.0.1:<traffic-port>/readyz
```

Current readiness means the admitted configuration and serving path are active; it is **not** an upstream-health guarantee. Health endpoints bypass the application in-flight admission budget so process health remains visible while application traffic is saturated.

See [`API_CONFIG_CONTRACT.md`](API_CONFIG_CONTRACT.md) for the configuration contract.

## Security and traffic behavior

The v1 gateway starts from distrust at the edge:

- inbound `Forwarded`, `X-Forwarded-*`, and `X-Real-IP` identity is discarded;
- the current cleartext downstream listener does not invent a trusted client IP;
- hop-by-hop and connection-nominated request headers are removed by the upstream policy;
- body, in-flight request, upstream keepalive-pool, and upstream-I/O budgets are explicit;
- metrics use low-cardinality labels and should be exposed only on an access-controlled observability network;
- access logs deliberately exclude URI/query, Host, client identity, authorization/cookie values, tokens, and configured credentials.

For the full threat and trust boundary, read [`SECURITY.md`](SECURITY.md) and [`THREAT_MODEL.md`](THREAT_MODEL.md).

## Container usage

The shipped `Dockerfile` packages the current candidate binary as numeric uid/gid `65532` and is designed for a read-only root filesystem with configuration mounted read-only. Rust 1.98.1 fixes a compiler miscompilation in 1.98.0; while Docker Official Image metadata still lacks a `1.98.1-bookworm` tag, the build keeps the reviewed digest-pinned 1.98.0 image only as a bootstrap environment and explicitly installs, selects, and verifies Rust 1.98.1 before compiling the gateway. That bridge must be replaced by a digest-pinned official 1.98.1 image when one is published.

This is an OCI **build/runtime contract**, not evidence of a published production image. No immutable registry digest, released image, customer cutover, or production availability claim exists unless separately backed by current release/deployment evidence.

## Integration maturity

The current v1 surface is intentionally narrow: one traffic listener, one metrics listener, and one upstream. Route tables, product-aware load balancing, WebSocket policy, dynamic reload, downstream TLS termination, ACME, Kubernetes Gateway API, and broader migration parity remain future increments that require a real consumer and executable acceptance evidence.

Existing Nginx or Traefik use in another repository is not automatically a Pingora migration candidate. Static serving, PHP/FastCGI, certificate management, application routing, and product-specific ingress may belong to other boundaries.

## Supply-chain and licensing posture

The crate's dependency policy permits a reviewed commercial-friendly set including Apache-2.0, MIT, BSD, ISC, CC0-1.0, OpenSSL, Unicode-3.0, and Zlib families; unknown registries/git sources and wildcard dependencies are denied. Pingora and `pingora-prometheus` are pinned to exact version `0.8.0` plus an immutable Cloudflare git revision.

Cloudflare Pingora is Apache-2.0 licensed. This repository's own crate metadata is also Apache-2.0, and the root [`LICENSE`](LICENSE) now carries that grant. Third-party dependencies retain their own license and attribution obligations; the repository license does not replace dependency provenance.

Current supply-chain policy still reports inherited maintenance concerns from the pinned framework rather than relabeling them as vulnerabilities or suppressing real vulnerability/unsoundness findings. See [`deny.toml`](deny.toml) and the current supply-chain workflow for the executable policy.

## Quality and status

This is a **0.1.0 candidate / Draft** product line, not a released gateway. The branch contains locked format/test/Clippy/doc builds, compiled-binary loopback E2E including saturation/recovery, OCI security acceptance, security/SAST lanes, and explicit supply-chain policy. Public Rust API documentation is a build gate via `#![deny(missing_docs)]` and `RUSTDOCFLAGS="-D warnings"`.

Run the core local checks with:

```bash
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked
```

Do not infer benchmark, production-traffic, certification, release, or migration claims from these engineering gates. The repository's product-gap baseline tracks the remaining evidence required before those claims are possible.

## Documentation map

- [`docs/index.md`](docs/index.md) — compact public documentation landing source.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — edge-runtime architecture and responsibility boundary.
- [`CONTEXT_MAP.md`](CONTEXT_MAP.md) — DDD context ownership and external authorities.
- [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md) — domain terminology.
- [`API_CONFIG_CONTRACT.md`](API_CONFIG_CONTRACT.md) — public gateway configuration contract.
- [`SECURITY.md`](SECURITY.md) / [`THREAT_MODEL.md`](THREAT_MODEL.md) — security posture and threats.
- [`TEST_STRATEGY.md`](TEST_STRATEGY.md) — executable verification strategy.
- [`OPERABILITY.md`](OPERABILITY.md) — runtime and operational guidance.
- [`docs/product-technical-gap-baseline.md`](docs/product-technical-gap-baseline.md) — current gaps, evidence, and next acceptance boundaries.
- [`docs/doctoring/TRACEABILITY.md`](docs/doctoring/TRACEABILITY.md) — primary-source traceability.

`docs/index.md` is only a Pages source prerequisite. Its presence does not mean GitHub Pages is published; publication requires protected integration, repository-settings reconciliation, successful deployment, and live HTTPS verification.

## Contributing

Keep the gateway generic and small. If a behavior requires product business knowledge, identity authority, certificate ownership, or domain retry semantics, implement it at the owning product boundary rather than teaching the shared edge runtime to guess.

New dependencies must permit commercial use under the intended distribution model and remain covered by the repository's explicit supply-chain policy. Update code, tests, architecture/security docs, and README claims together when a public edge contract changes.

## License

Pingora Gateway is licensed under the [Apache License 2.0](LICENSE). Third-party components retain their applicable licenses and attribution terms.
