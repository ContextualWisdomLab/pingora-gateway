# Cutover observability traceability

Last verified: 2026-09-11

This note covers the transport-only telemetry needed to distinguish the Pingora TLS/H2 migration path from legacy or cleartext/H1 traffic during parity, shadow, canary, rollback and cutover. It does not create deployment, product, identity or security-verdict authority. `docs/product-technical-gap-baseline.md` remains owned by dedicated documentation lane #61.

## Problem and decision

Before PR #88, the shared gateway emitted aggregate request/error/body-byte counters and payload-free completion logs. Those signals could show that the process handled requests, but they could not establish that a canary request actually traversed TLS/H2 rather than H1/cleartext. A mixed-protocol migration could therefore hide an H2-specific regression inside healthy aggregate traffic.

PR #88 adds one counter family, `cwl_pingora_gateway_requests_by_transport_total`, with exactly three bounded labels:

- `outcome`: `ok` or `error`;
- `protocol`: `h1` or `h2`;
- `transport`: `cleartext` or `tls`.

The Cartesian product is eight possible time series. Protocol is derived from the accepted Pingora server session. TLS presence is derived from the session transport digest populated from the underlying stream. No request-controlled header participates in classification.

Rejected alternatives were route, hostname, client address, certificate path, customer/product identity, deployment revision and arbitrary error strings as labels. They are either outside gateway authority, privacy-sensitive, deployment-owned, or unbounded/high-cardinality. Prometheus explicitly warns that each unique label set creates a separate time series and recommends keeping label cardinality small.

The existing aggregate counters remain unchanged. The transport counter is an additional cutover discriminator, not a replacement for latency, TLS-handshake, release identity, rollback, product correctness or security evidence.

## Executable evidence

The real-wire acceptance in `tests/downstream_tls_h2_wire.rs` establishes a CA-verified downstream TLS connection, negotiates ALPN `h2`, sends an actual HTTP/2 request, proxies it to the existing cleartext HTTP/1 origin fixture, waits for normal stream completion, then scrapes the production Prometheus listener. It requires the exact sample:

`cwl_pingora_gateway_requests_by_transport_total{outcome="ok",protocol="h2",transport="tls"} 1`

The RED-only head `701faf4390225ae650870dedf64dc921e838e278` added that requirement without changing production code. Its hosted run was superseded while compile/test was executing, so it is test intent and local causal evidence, not a terminal hosted RED receipt. Production increment `f0231178a0eb21c66b3f81b6ed1239cb608ff6c1` added bounded classification but CI `34601751211` stopped at Rust 1.98.0 formatting before compilation. Formatter-only repair `7834aad1c6b991e57b663590e240489ef06a4446` applies exactly the emitted rustfmt changes.

Exact `cc9067006296b6dd1fb5f89d2b26fe5ea4e9e77a` then passed formatting, compile/test including the real-wire metric assertion, Clippy, warnings-denied rustdoc, load-contract, OCI runtime, Supply Chain and bounded-origin capacity. CI `34602099784` failed only at the final complete-owned-production coverage gate. Coverage artifact `coverage-cc9067006296b6dd1fb5f89d2b26fe5ea4e9e77a` (`sha256:39bb9923363809341feca489bdecfd95ffec1f0f355e84305b88be13021c13d2`) isolated the only file below 100% to `src/observability.rs`; the two zero-count regions were the duplicate-registration panic continuation created directly inside the transport CounterVec `LazyLock`. Ordinary-forward repair `2e2674033667007872be7bc95261f1eb60f5e0e0` factors CounterVec registration through the same fail-closed helper shape already used for scalar counters and adds a duplicate-vector registration regression. It does not change metric names, labels, pre-created series, transport classification or failure semantics.

A fresh fixture review also found that this older H2 wire test released the traffic and metrics port reservations immediately after discovering their ephemeral addresses. That created a TOCTOU window in which an unrelated process could bind either port between configuration generation and gateway spawn, producing a non-protocol RED. Ordinary-forward `5cdc84284f1eebde932d654c742ca4159ed54216` adopts the already-proven H2 fixture pattern: both `TcpListener` reservations remain open until immediately before spawning the gateway, then are dropped inside `spawn_gateway`. The real-wire H2/TLS, upstream-proxy and metric oracles are unchanged.

Any later documentation movement still creates a new exact head. No predecessor receipt is transferred: the final PR #88 exact head must independently pass CI, Supply Chain, bounded-origin capacity, review-thread and current-head review gates before Ready promotion.

## Supplier and standards traceability

Pingora release authority remains 0.9.0 at source `702f69015e53f7244d6ad2e743de571d859a70a4`. Its HTTP server session exposes whether the negotiated request session is HTTP/2, and its accepted server-session digest carries the TLS digest populated from the underlying transport. PR #88 consumes those released observations rather than parsing request headers or copying supplier internals.

RFC 9113 defines HTTP/2 protocol semantics; the telemetry label reports the protocol already admitted by Pingora and does not reinterpret H2 frames. TLS/H2 acceptance remains governed by the existing downstream TLS/H2 traceability note.

Prometheus metric guidance recommends labels for dimensions of one metric while warning that each unique label combination creates a separate time series. Its instrumentation guidance advises avoiding label overuse and keeping cardinality small. The eight-series contract here is deliberately finite and independent of traffic data.

## Operational use and limits

During shadow/canary, operators can compare `ok` and `error` rates for `protocol="h2",transport="tls"` separately from H1 or cleartext traffic. A nonzero successful sample proves that completed traffic reached the intended transport class; it does not prove that the deployment is the intended immutable release, that p95 is within target, that TLS handshakes are efficient, or that rollback has been rehearsed. Those remain separate promotion evidence.

The metric must not gain route, tenant, user, source-IP, destination-IP, certificate, repository SHA, environment, customer or product labels without a separately reviewed bounded-cardinality and authority analysis. Product-domain telemetry belongs with the product owner, and identity/security verdicts remain with Keyverse/Wardnet/EgressWeave as applicable.

## Primary references (APA 7th)

Cloudflare, Inc. (2026). *Pingora HTTP server protocol implementation* (release 0.9.0, source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/server.rs

Cloudflare, Inc. (2026). *Pingora application session digest construction* (release 0.9.0, source `702f69015e53f7244d6ad2e743de571d859a70a4`). https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/apps/mod.rs

Prometheus Authors. (2026). *Instrumentation*. Prometheus. https://prometheus.io/docs/practices/instrumentation/

Prometheus Authors. (2026). *Metric and label naming*. Prometheus. https://prometheus.io/docs/practices/naming/

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). Internet Engineering Task Force. https://doi.org/10.17487/RFC9113
