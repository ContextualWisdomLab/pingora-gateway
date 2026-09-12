# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

For the current candidate dependency graph, Pingora release semantics are authoritative at tag `0.9.0`, which resolves to commit `702f69015e53f7244d6ad2e743de571d859a70a4`. Cloudflare's protected `main` is prospective supplier evidence only; on 2026-09-12 it was `4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19` and must not silently replace released dependency authority. Historical 0.8-line source remains background only where explicitly identified as such.

| Claim | Source |
| --- | --- |
| Current Pingora release authority for the candidate branch is `0.9.0@702f69015e53f7244d6ad2e743de571d859a70a4`; moving upstream `main` is not release authority | Cloudflare Pingora Git tag `0.9.0` and GitHub Release metadata, revalidated 2026-09-12 |
| Pingora server/proxy composition and graceful server lifecycle remain present on the consumed 0.9.0 release line | `pingora-core/src/server/mod.rs`, `pingora-core/src/server/configuration/mod.rs`, and `pingora-proxy/src/lib.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4` |
| Pingora diagnostics can contain request-derived data outside CWL's callback logging vocabulary: core/proxy TRACE paths format request headers before the gateway can treat those messages as bounded observability | `pingora-core/src/apps/http_app.rs` and `pingora-proxy/src/lib.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; this remains the source basis for process-wide Pingora-family diagnostic message redaction in `logging_policy` |
| Pingora downstream sessions expose accepted client/server socket addresses; Pingora socket addresses expose IP socket values through `as_inet()` | `pingora-core/src/protocols/http/server.rs`, `pingora-proxy/tests/utils/server_utils.rs`, and `pingora-core/src/protocols/l4/socket.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; these are the transport observations used by the pg-erd forwarding adapter |
| Pingora's server default `max_retries` is 16, while the proxy stores the configured value for retry handling | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; CWL sets the field to `1` for the bounded one-attempt contract |
| Graceful shutdown configuration retains `grace_period_seconds` and `graceful_shutdown_timeout_seconds`; 0.9.0 also changes shutdown/upgrade internals, so the current release source rather than the historical 0.8-line snapshot is the authority | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; Pingora 0.9.0 release notes |
| Pingora 0.9.0's upstream request policy strips standard hop-by-hop and `Connection`-nominated fields by default, forwards valid WebSocket upgrade handshakes in normalized form, and exposes explicit compatibility settings; supplier capability is not itself an admitted CWL protocol-transition contract | `docs/user_guide/peer.md` and `pingora-core/src/upstreams/peer.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; generic and bounded migration profiles continue to fail closed on protocol transitions not admitted by their versioned contracts |
| Pingora 0.9.0 HTTP/1 request-header admission is finite but supplier-fixed: `MAX_HEADERS = 256`, `INIT_HEADER_BUF_SIZE = 4096`, and `MAX_HEADER_SIZE = 1_048_575`; the public `HttpServerOptions` surface has no HTTP/1 header byte/count admission field | `pingora-core/src/protocols/http/v1/common.rs`, `pingora-core/src/protocols/http/v1/server.rs`, and `pingora-core/src/apps/mod.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; downstream issue `pingora-gateway#43` and supplier work track the missing composition hook |
| Pingora 0.9.0 downstream HTTP/1 request-header acquisition keeps a 60 s default `read_timeout`, applied to individual socket reads while `read_request()` can loop until a complete header parses. A successful partial read can therefore receive another relative read budget, and `HttpServerOptions` exposes no monotonic whole-header deadline | `pingora-core/src/protocols/http/v1/server.rs`, `pingora-proxy/src/lib.rs`, and `pingora-core/src/apps/mod.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; supplier issue `cloudflare/pingora#447` and `pingora-gateway#45` track the whole-header lifetime boundary |
| Pingora HTTP/2 request-header admission uses separate decoded-header-list accounting: 0.9.0 `default_h2_options()` sets a 64 KiB decoded header-list limit and 100 concurrent streams through `H2Options`; these semantics must not be reinterpreted as HTTP/1 wire/parser bytes | `pingora-core/src/protocols/http/v2/server.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; Pingora 0.9.0 release notes identify bounded default HTTP/2 limits as a breaking/hardening change |
| Pingora `read_timeout` is a per-individual-read inactivity budget and resets after each successful upstream `read()`; it is not a total-response lifetime bound | Cloudflare Pingora `docs/user_guide/peer.md` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; the pg-erd read-stall acceptance therefore keeps a connected origin silent without closing its socket and deliberately does not claim slow-drip/whole-response bounding |
| A proxy failure after the upstream response header has already been sent downstream cannot be replaced with a new error response or failover | Cloudflare Pingora `docs/user_guide/failover.md` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; the pg-erd partial-response fixture therefore observes the committed status/body prefix before causing origin termination and requires the existing downstream response to terminate rather than become a second response |
| Pingora HTTP/1 body framing treats a body that ends before its declared `Content-Length` as a premature body-end failure, and 0.9.0 additionally hardens HTTP/1 reuse after incomplete responses | `pingora-core/src/protocols/http/v1/body.rs`, `pingora-core/src/protocols/http/v1/client.rs`, and `pingora-proxy/src/proxy_h1.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; Pingora 0.9.0 release notes; RFC 9112 message-framing requirements |
| Pingora's Prometheus HTTP application is a separate 0.9.0 crate and sets `Content-Type` from `prometheus::TextEncoder::format_type()`; the pinned `prometheus` 0.14 line defines that format as `text/plain; version=0.0.4` | `pingora-prometheus/src/lib.rs` at release commit `702f69015e53f7244d6ad2e743de571d859a70a4` and TiKV `rust-prometheus` v0.14.0 commit `e07efb4f372f1245bf7410b71e822c69877bcb32`, `src/encoder/text.rs`; #19 therefore strips only optional semicolon parameters and requires exact base media type `text/plain` rather than a prefix wildcard |
| Pingora 0.9.0 peers support per-peer CA configuration; certificate and hostname verification remain independently represented in peer options | `pingora-core/src/upstreams/peer.rs` and the TLS connector/protocol implementations at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; Pingora 0.9.0 release notes list per-peer CA configuration as a TLS feature |
| The consumed Pingora 0.9.0 source still imports and uses `derivative 2.2.0`; RustSec classifies `RUSTSEC-2024-0388` as an informational unmaintained-package advisory with no patched versions. This remains a release supplier root rather than an advisory to suppress | `pingora-core/src/upstreams/peer.rs` and workspace dependency declarations at release commit `702f69015e53f7244d6ad2e743de571d859a70a4`; RustSec `RUSTSEC-2024-0388`; `cloudflare/pingora#889` |
| IPv4-mapped IPv6 addresses represent IPv4 nodes in IPv6 form; Rust's `Ipv6Addr::to_ipv4_mapped` performs the bounded mapped-only canonicalization used by the socket-authority invariant. Linux IPv6 sockets can expose IPv4 peers as mapped IPv6 addresses, so mapped/native authority must not be compared only as unrelated textual address families | RFC 4291 §2.5.5.2; Rust `std::net::Ipv6Addr` documentation; Linux `ipv6(7)` |
| Traefik normally adds `X-Forwarded-For`, `X-Real-Ip`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, and `X-Forwarded-Server` when proxying HTTP | Traefik official Getting Started FAQ, current documentation revalidated 2026-09-02 |
| Incoming Traefik `X-Forwarded-*` identity is trusted only when an EntryPoint explicitly configures trusted IPs or insecure trust; insecure mode is not recommended for production | Traefik official EntryPoints documentation, current documentation revalidated 2026-09-02 |
| `pg-erd-cloud` can use `X-Forwarded-For` for rate-limit/observability client identity only under an explicit trust switch and tells operators to enable it only behind a sanitizing ingress | `ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535`: `.env.example`, `backend/app/rate_limit.py`, `backend/app/observability.py`, `docs/api-security-checklist.md` |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics, including the HTTP/1 Upgrade field and a server's ability to decline a requested transition | RFC 9110 §7.8 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| WebSocket over HTTP/1 uses an opening handshake; after sending that handshake the client must wait for the server response before sending WebSocket data | RFC 6455 §4.1; this supports fail-closed admission and prevents treating an Upgrade request as already-authorized tunnel traffic |
| WebSocket over HTTP/2 is bootstrapped with Extended CONNECT rather than HTTP/1 Upgrade semantics | RFC 8441; this remains a separate protocol contract from the current HTTP/1 fail-closed admission rule |
| Optimistic HTTP/1.1 protocol-transition data can create request-smuggling/parser risks when a transition is rejected; current guidance adds normative requirements to HTTP/1.1 and explicitly notes that WebSocket already forbids optimistic post-handshake data | RFC 9931, published March 2026, updating RFC 9112 and RFC 9298 |
| HTTP/2 framing and connection semantics | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented until executable listener/interoperability evidence exists |
| Current TLS 1.3 protocol semantics and application identity-verification responsibility | RFC 9846, published July 2026, which obsoletes RFC 8446 and points applications to RFC 9525 for identity verification |
| New protocols using TLS must require TLS 1.3 | RFC 9852, BCP 195, July 2026; this gateway is not claiming a new application protocol and still requires explicit migration-time protocol compatibility evidence |
| March 2026 Pingora request-smuggling/cache-key advisories were patched before the current 0.9.0 release line | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc; Pingora 0.9.0 release authority remains the candidate baseline |
| Pingora 0.9.0 is the latest published release observed on 2026-09-12; GitHub published the release on 2026-09-09, and tag `0.9.0` resolves to `702f69015e53f7244d6ad2e743de571d859a70a4` | Cloudflare Pingora GitHub Release `0.9.0` and Git tag `0.9.0` |
| Cloudflare protected `main` was `4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19` when revalidated on 2026-09-12; it is prospective supplier evidence only and cannot replace a maintainer-integrated, release-qualified identity | Cloudflare Pingora protected branch metadata; compare/review only after an upstream release or other approved immutable supplier identity exists |
| Rust 1.98.1 is the current stable point release observed on 2026-09-07 and fixes a Rust 1.98.0 vtable-generation miscompilation that could place a null pointer where a trait-object function pointer was required | Rust Release Team, Rust 1.98.1 announcement, 2026-09-03; this external authority does not by itself promote the repository's separately gated compiler prerequisite |
| OCI runtime-spec 1.3.0 is the latest released runtime specification observed on 2026-09-07 | Open Container Initiative runtime-spec v1.3.0 release notice, 2025-11-04; runtime hardening claims still require executable container evidence |
| OCI image-spec 1.1.1 is the latest released image specification observed on 2026-09-07 | Open Container Initiative image-spec v1.1.1 release notice, 2025-04-02; image-format conformance does not prove non-root, read-only-root, capability or `no-new-privileges` runtime behavior, which remains an executable acceptance concern |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; release qualification must use the repository's committed, audited lock and must not infer dependency safety from a moving upstream branch |

## References

Cloudflare. (2026, September 9). *Pingora 0.9.0*. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

Cloudflare. (n.d.). *Pingora upstream peer options* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/upstreams/peer.rs

Cloudflare. (n.d.). *Peer: how to connect to upstream* [Documentation, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/docs/user_guide/peer.md

Cloudflare. (n.d.). *Handling failures and failover* [Documentation, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/docs/user_guide/failover.md

Cloudflare. (n.d.). *Pingora HTTP/1 request-header parser limits* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v1/common.rs

Cloudflare. (n.d.). *Pingora HTTP/1 server session* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v1/server.rs

Cloudflare. (n.d.). *Pingora HTTP server options* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/apps/mod.rs

Cloudflare. (n.d.). *Pingora HTTP/2 server session and bounded defaults* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v2/server.rs

seonghobae. (2026, September 3). *Expose configurable HTTP/1 request-header parser admission limits* [GitHub issue #993]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/993

Ermakov, O. (2024, October 29). *Support for Configurable Timeout for Downstream Connections in HTTP/1.1* [GitHub issue #447]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/447

Cloudflare. (n.d.). *Pingora HTTP/1 proxy implementation* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-proxy/src/proxy_h1.rs

Cloudflare. (n.d.). *Pingora HTTP/1 client session* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v1/client.rs

Cloudflare. (n.d.). *Pingora HTTP/1 body framing* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/v1/body.rs

Cloudflare. (n.d.). *Pingora Prometheus HTTP application* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-prometheus/src/lib.rs

Cloudflare. (n.d.). *Pingora OpenSSL upstream TLS connector* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/connectors/tls/boringssl_openssl/mod.rs

Cloudflare. (n.d.). *Pingora downstream HTTP session* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/http/server.rs

Cloudflare. (n.d.). *Pingora L4 socket address* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/protocols/l4/socket.rs

Cloudflare. (n.d.). *Pingora server configuration* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/configuration/mod.rs

Cloudflare. (n.d.). *Pingora server lifecycle* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-core/src/server/mod.rs

Cloudflare. (n.d.). *Pingora proxy implementation* [Source code, release commit 702f69015e53f7244d6ad2e743de571d859a70a4]. GitHub. https://github.com/cloudflare/pingora/blob/702f69015e53f7244d6ad2e743de571d859a70a4/pingora-proxy/src/lib.rs

Cloudflare. (2026). *HTTP request smuggling via premature upgrade* (GHSA-xq2h-p299-vjwv). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-xq2h-p299-vjwv

Cloudflare. (2026). *HTTP request smuggling via HTTP/1.0 and Transfer-Encoding misparsing* (GHSA-hj7x-879w-vrp7). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-hj7x-879w-vrp7

Cloudflare. (2026). *Cache key poisoning advisory* (GHSA-f93w-pcj3-rggc). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-f93w-pcj3-rggc

TiKV Project Authors. (2025). *rust-prometheus 0.14.0 text encoder* [Source code, commit e07efb4f372f1245bf7410b71e822c69877bcb32]. GitHub. https://github.com/tikv/rust-prometheus/blob/e07efb4f372f1245bf7410b71e822c69877bcb32/src/encoder/text.rs

Hinden, R., & Deering, S. (2006). *IP version 6 addressing architecture* (RFC 4291). RFC Editor. https://www.rfc-editor.org/rfc/rfc4291

The Rust Project Developers. (2026). *Ipv6Addr in std::net*. Rust standard library documentation. https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html

Kerrisk, M. (n.d.). *ipv6(7) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man7/ipv6.7.html

Traefik Labs. (n.d.). *Traefik getting started FAQ: Forwarded headers when proxying HTTP requests*. https://doc.traefik.io/traefik/getting-started/faq/

Traefik Labs. (n.d.). *Traefik EntryPoints: Forwarded headers*. https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Fette, I., & Melnikov, A. (2011). *The WebSocket protocol* (RFC 6455). RFC Editor. https://www.rfc-editor.org/rfc/rfc6455

McManus, P. (2018). *Bootstrapping WebSockets with HTTP/2* (RFC 8441). RFC Editor. https://www.rfc-editor.org/rfc/rfc8441

Schwartz, B. M. (2026). *Security considerations for optimistic protocol transitions in HTTP/1.1* (RFC 9931). RFC Editor. https://www.rfc-editor.org/rfc/rfc9931

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Open Container Initiative. (2025, April 2). *OCI image-spec v1.1.1 release notice*. https://opencontainers.org/release-notices/v1-1-1-image-spec/

Open Container Initiative. (2025, November 4). *OCI runtime-spec v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Rust Secure Code Working Group. (2024, November 10). *RUSTSEC-2024-0388: derivative is unmaintained; consider using an alternative*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2024-0388.html

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
