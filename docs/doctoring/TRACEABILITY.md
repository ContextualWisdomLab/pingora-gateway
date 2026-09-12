# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version, branch, release, and advisory claims immediately before promotion or release.

| Claim | Source |
| --- | --- |
| The current gateway stack is compiled against exact Pingora source `09696b51bc59315353d96686355861604d0bb48c`; that SHA is a candidate source pin, not current upstream protected-branch authority | `Cargo.toml`/`Cargo.lock` in the current gateway stack plus Cloudflare Pingora source at commit `09696b51bc59315353d96686355861604d0bb48c` |
| Cloudflare Pingora protected `main` observed on 2026-09-13 is `4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19`; the workspace at that exact still declares `derivative = "2.2.0"` | Cloudflare Pingora GitHub branch API and root `Cargo.toml` at `4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19` |
| Pingora 0.9.0 is the latest published release observed on 2026-09-13; GitHub published it on 2026-09-09 and its release notes label the release 2026-09-04. The release is not immutable and has no attached assets, so recency alone is not CWL release authority | Cloudflare Pingora GitHub Release `0.9.0` |
| Pingora server/proxy composition and graceful server lifecycle used by the current candidate | Cloudflare Pingora source at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Pingora downstream sessions expose accepted client/server socket addresses; Pingora socket addresses expose IP socket values through `as_inet()` | `pingora-core/src/protocols/http/server.rs`, `pingora-proxy/tests/utils/server_utils.rs`, and `pingora-core/src/protocols/l4/socket.rs` at candidate commit `09696b51bc59315353d96686355861604d0bb48c`; these are the transport observations used by the pg-erd forwarding adapter |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at candidate commit `09696b51bc59315353d96686355861604d0bb48c`; CWL v1 sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the candidate commit; CWL v1 sets 5 s grace and 10 s per-runtime graceful timeout explicitly inside a 30 s external termination budget |
| Standard upstream request policy supports hop-by-hop/connection-nominated stripping and normalized WebSocket-only HTTP/1 upgrade forwarding | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at candidate commit `09696b51bc59315353d96686355861604d0bb48c` |
| Pingora OpenSSL peers support a per-peer CA store; when configured it replaces the verification store for that peer while certificate and hostname verification remain separately enabled | `pingora-core/src/upstreams/peer.rs`, `pingora-core/src/connectors/tls/boringssl_openssl/mod.rs`, and `pingora-core/src/protocols/tls/boringssl_openssl/mod.rs` at candidate commit `09696b51bc59315353d96686355861604d0bb48c` |
| Traefik normally adds `X-Forwarded-For`, `X-Real-Ip`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, and `X-Forwarded-Server` when proxying HTTP | Traefik official Getting Started FAQ, revalidated 2026-09-13 |
| Incoming Traefik `X-Forwarded-*` identity is trusted only when an EntryPoint explicitly configures trusted IPs or insecure trust; insecure mode is not recommended for production | Traefik official EntryPoints documentation, revalidated 2026-09-13 |
| `pg-erd-cloud` can use `X-Forwarded-For` for rate-limit/observability client identity only under an explicit trust switch and tells operators to enable it only behind a sanitizing ingress | `ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535`: `.env.example`, `backend/app/rate_limit.py`, `backend/app/observability.py`, `docs/api-security-checklist.md` |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics and field-content requirements | RFC 9110 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP/2 framing and connection semantics | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented by this candidate until executable listener/interoperability evidence exists |
| Current TLS 1.3 protocol semantics and application identity-verification responsibility | RFC 9846, published July 2026, which obsoletes RFC 8446 and points applications to RFC 9525 for identity verification |
| New protocols using TLS must require TLS 1.3 | RFC 9852, BCP 195, July 2026; this gateway is not claiming a new application protocol and still requires explicit migration-time protocol compatibility evidence |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| `derivative 2.2.0` is unmaintained under RUSTSEC-2024-0388 and remains present in current upstream protected `main`; supplier promotion therefore remains fail-closed until a maintainer-integrated, release-qualified disposition exists | RustSec RUSTSEC-2024-0388; Cloudflare Pingora issue #889; Cloudflare `Cargo.toml@4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19` |
| The current gateway CI stack still runs Rust 1.98.0, while Rust 1.98.1 is the latest stable release observed on 2026-09-13 and fixes a vtable-generation miscompilation in 1.98.0 that could emit undefined behavior | Rust Release Team, Rust 1.98.1 announcement, 2026-09-03; compiler promotion is owned by gateway PR #56 and is not silently folded into this migration callback slice |
| OCI runtime-spec 1.3.0 is the latest released runtime specification observed in the current gateway documentation | Open Container Initiative runtime-spec v1.3.0 release notice, 2025-11-04; runtime hardening claims still require executable container evidence |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; supplier promotion must use a committed audited lock rather than infer safety from a moving upstream branch |

## References

Cloudflare. (2026, September 9). *Pingora 0.9.0*. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

Cloudflare. (n.d.). *Pingora upstream peer options* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/upstreams/peer.rs

Cloudflare. (n.d.). *Pingora OpenSSL upstream TLS connector* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/connectors/tls/boringssl_openssl/mod.rs

Cloudflare. (n.d.). *Pingora downstream HTTP session* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/server.rs

Cloudflare. (n.d.). *Pingora L4 socket address* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/l4/socket.rs

Cloudflare. (n.d.). *Pingora server configuration* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/configuration/mod.rs

Cloudflare. (n.d.). *Pingora server lifecycle* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/mod.rs

Cloudflare. (n.d.). *Pingora proxy implementation* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

Cloudflare. (2026). *HTTP request smuggling via premature upgrade* (GHSA-xq2h-p299-vjwv). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-xq2h-p299-vjwv

Cloudflare. (2026). *HTTP request smuggling via HTTP/1.0 and Transfer-Encoding misparsing* (GHSA-hj7x-879w-vrp7). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-hj7x-879w-vrp7

Cloudflare. (2026). *Cache key poisoning advisory* (GHSA-f93w-pcj3-rggc). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-f93w-pcj3-rggc

Traefik Labs. (n.d.). *Traefik getting started FAQ: Forwarded headers when proxying HTTP requests*. https://doc.traefik.io/traefik/getting-started/faq/

Traefik Labs. (n.d.). *Traefik EntryPoints: Forwarded headers*. https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Open Container Initiative. (2025, November 4). *OCI runtime-spec v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/releases/1.98.1/

Rust Secure Code Working Group. (2024). *RUSTSEC-2024-0388: derivative is unmaintained*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2024-0388.html

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
